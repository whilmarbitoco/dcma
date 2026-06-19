use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractedEntity {
    pub name: String,
    pub entity_type: String,
    pub confidence: f64,
    pub position: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractedRelation {
    pub src_idx: usize,
    pub dst_idx: usize,
    pub relation_type: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResult {
    pub entities_found: usize,
    pub entities_deduplicated: usize,
    pub relations_found: usize,
    pub atoms_created: usize,
    pub relations_created: usize,
}

pub struct IngestEngine {
    pub min_entity_confidence: f64,
    pub min_relation_confidence: f64,
    pub cooccurrence_window: usize,
    pub deduplicate: bool,
}

impl Default for IngestEngine {
    fn default() -> Self {
        Self {
            min_entity_confidence: 0.3,
            min_relation_confidence: 0.2,
            cooccurrence_window: 10,
            deduplicate: true,
        }
    }
}

impl IngestEngine {
    pub fn new() -> Self {
        Self::default()
    }

    fn is_upper(c: char) -> bool {
        c.is_ascii_uppercase()
    }

    fn is_lower(c: char) -> bool {
        c.is_ascii_lowercase()
    }

    fn is_word_char(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_' || c == '-'
    }

    pub fn extract_entities(&self, text: &str) -> Vec<ExtractedEntity> {
        let mut entities = Vec::new();
        let mut seen: HashMap<String, usize> = HashMap::new();
        let chars: Vec<(usize, char)> = text.char_indices().collect();
        let len = chars.len();
        let mut i = 0;
        // Track which byte positions are already covered by entities
        let mut covered: Vec<(usize, usize)> = Vec::new();

        while i < len {
            let (pos, c) = chars[i];

            if Self::is_upper(c) {
                let scan_start = if i + 1 < len && Self::is_lower(chars[i + 1].1) {
                    let word_end = self.find_word_end(&chars, i);
                    let first_word: String = chars[i..word_end].iter().map(|(_, ch)| *ch).collect();
                    if self.is_common_word(&first_word) && word_end < len && chars[word_end].1 == ' ' {
                        let next_start = word_end + 1;
                        if next_start < len && Self::is_upper(chars[next_start].1) {
                            next_start
                        } else {
                            i = word_end + 1;
                            continue;
                        }
                    } else {
                        i
                    }
                } else if i + 1 < len && Self::is_upper(chars[i + 1].1) {
                    match self.scan_all_caps(&chars, i) {
                        Some((name, etype, end_pos)) => {
                            if name.len() >= 2 {
                                let entity_type = self.classify_type(&name, &etype);
                                self.add_entity(&mut entities, &mut seen, name.clone(), &entity_type, self.confidence_for(&etype), pos);
                                covered.push((pos, pos + name.len()));
                            }
                            i = end_pos;
                            continue;
                        }
                        None => { i += 1; continue; }
                    }
                } else {
                    i += 1;
                    continue;
                };

                match self.scan_capitalized_from(&chars, scan_start, text) {
                    Some((name, etype, end_pos)) => {
                        if name.len() >= 2 {
                            let entity_type = self.classify_type(&name, &etype);
                            self.add_entity(&mut entities, &mut seen, name.clone(), &entity_type, self.confidence_for(&etype), pos);
                            covered.push((pos, pos + name.len()));
                        }
                        i = end_pos;
                    }
                    None => { i += 1; }
                }
            } else {
                i += 1;
            }
        }

        // Lowercase noun phrases (skip positions already covered by entities)
        self.extract_lowercase_phrases(text, &mut entities, &mut seen, &covered);

        // Quoted terms
        for (name, pos) in self.scan_quoted(text) {
            if name.len() > 1 {
                self.add_entity(&mut entities, &mut seen, name, "Concept", 0.7, pos);
            }
        }

        entities.retain(|e| e.confidence >= self.min_entity_confidence);
        entities.sort_by_key(|e| e.position);
        entities
    }

    pub fn extract_relations(&self, entities: &[ExtractedEntity], text: &str) -> Vec<ExtractedRelation> {
        let mut relations = Vec::new();
        if entities.len() < 2 {
            return relations;
        }

        let mut sorted: Vec<usize> = (0..entities.len()).collect();
        sorted.sort_by_key(|&i| entities[i].position);

        for i in 0..sorted.len() {
            for j in (i + 1)..sorted.len() {
                let pi = entities[sorted[i]].position;
                let pj = entities[sorted[j]].position;
                if pj > pi {
                    let between = &text[pi..pj];
                    if between.split_whitespace().count() <= self.cooccurrence_window {
                        relations.push(ExtractedRelation {
                            src_idx: sorted[i],
                            dst_idx: sorted[j],
                            relation_type: "CoOccursWith".to_string(),
                            confidence: 0.3,
                        });
                    }
                }
            }
        }

        let text_lower = text.to_lowercase();
        let patterns: Vec<(&str, &str, f64)> = vec![
            (" is a ", "InstanceOf", 0.9),
            (" is an ", "InstanceOf", 0.9),
            (" are a ", "InstanceOf", 0.9),
            (" are an ", "InstanceOf", 0.9),
            (" is the ", "InstanceOf", 0.85),
            (" has ", "Contains", 0.7),
            (" have ", "Contains", 0.7),
            (" contains ", "Contains", 0.8),
            (" causes ", "Causes", 0.9),
            (" leads to ", "Causes", 0.85),
            (" prevents ", "Prevents", 0.9),
            (" blocks ", "Prevents", 0.8),
            (" supports ", "Supports", 0.8),
            (" enables ", "Enables", 0.85),
            (" contradicts ", "Contradicts", 0.9),
            (" opposes ", "Contradicts", 0.8),
            (" is not ", "Negates", 0.85),
            (" implies ", "Implies", 0.85),
            (" triggers ", "Triggers", 0.85),
            (" activates ", "Activates", 0.85),
            (" inhibits ", "Inhibits", 0.85),
            (" reinforces ", "Reinforces", 0.85),
        ];

        for (pattern, rel_type, conf) in &patterns {
            let mut start = 0;
            while let Some(pos) = text_lower[start..].find(pattern) {
                let abs_pos = start + pos;
                let center = abs_pos + pattern.len() / 2;

                let mut best_before: Option<usize> = None;
                let mut best_after: Option<usize> = None;
                let mut min_before = usize::MAX;
                let mut min_after = usize::MAX;

                for (idx, ent) in entities.iter().enumerate() {
                    let ent_end = ent.position + ent.name.len();
                    if ent_end <= center {
                        let dist = center.saturating_sub(ent_end);
                        if dist < min_before {
                            min_before = dist;
                            best_before = Some(idx);
                        }
                    } else if ent.position >= center {
                        let dist = ent.position - center;
                        if dist < min_after {
                            min_after = dist;
                            best_after = Some(idx);
                        }
                    }
                }

                if let (Some(src), Some(dst)) = (best_before, best_after) {
                    relations.push(ExtractedRelation {
                        src_idx: src,
                        dst_idx: dst,
                        relation_type: rel_type.to_string(),
                        confidence: *conf,
                    });
                }

                start = abs_pos + pattern.len().max(1);
                if start >= text_lower.len() { break; }
            }
        }

        relations.retain(|r| r.confidence >= self.min_relation_confidence);
        self.dedup_relations(relations)
    }

    pub fn ingest(
        &self,
        text: &str,
        atoms_out: &mut Vec<(String, String)>,
        relations_out: &mut Vec<(usize, usize, String, f64)>,
    ) -> IngestResult {
        let entities = self.extract_entities(text);
        let ec = entities.len();
        for e in &entities {
            atoms_out.push((e.name.clone(), e.entity_type.clone()));
        }
        let relations = self.extract_relations(&entities, text);
        let rc = relations.len();
        for r in &relations {
            relations_out.push((r.src_idx, r.dst_idx, r.relation_type.clone(), r.confidence));
        }
        IngestResult {
            entities_found: ec,
            entities_deduplicated: ec,
            relations_found: rc,
            atoms_created: atoms_out.len(),
            relations_created: relations_out.len(),
        }
    }

    // --- Scanners ---

    /// Scan a capitalized/CamelCase word from `start`.
    /// Handles: "Neural Networks", "Backpropagation", "BatchNormalization", "ReLU"
    fn scan_capitalized_from(
        &self,
        chars: &[(usize, char)],
        start: usize,
        _text: &str,
    ) -> Option<(String, String, usize)> {
        let len = chars.len();
        if start >= len || !Self::is_upper(chars[start].1) {
            return None;
        }

        let mut end = start;
        let mut has_camel = false;

        // Consume first word, handling CamelCase including ALL_CAPS suffixes
        // e.g., "ReLU" -> R, e, L, U  (upper, lower, upper, upper)
        // e.g., "BatchNormalization" -> B, a, t, c, h, N, o, r, m, ...
        end += 1; // skip first uppercase
        while end < len {
            if Self::is_lower(chars[end].1) {
                end += 1;
            } else if Self::is_upper(chars[end].1) {
                // Check what follows this uppercase char
                if end + 1 < len && Self::is_lower(chars[end + 1].1) {
                    // Next is lowercase → CamelCase transition (e.g., "BatchN")
                    has_camel = true;
                    end += 1;
                } else if end + 1 < len && Self::is_upper(chars[end + 1].1) {
                    // Next is also uppercase → could be ALL_CAPS suffix like "LU" in "ReLU"
                    // Consume consecutive uppercase chars as part of this word
                    end += 1;
                    while end < len && Self::is_upper(chars[end].1) {
                        end += 1;
                    }
                    // After consuming uppercase run, if next is lowercase, continue CamelCase
                    if end < len && Self::is_lower(chars[end].1) {
                        has_camel = true;
                        end += 1;
                        while end < len && Self::is_lower(chars[end].1) {
                            end += 1;
                        }
                    }
                    break;
                } else {
                    // End of string or non-letter after uppercase
                    break;
                }
            } else {
                break;
            }
        }

        // Consume subsequent space-separated capitalized words (e.g., "Neural Networks")
        while end < len && chars[end].1 == ' ' {
            let next = end + 1;
            if next < len && Self::is_upper(chars[next].1) {
                if next + 1 < len && Self::is_lower(chars[next + 1].1) {
                    let word_end = self.scan_camel_word(chars, next);
                    end = word_end;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        let name: String = chars[start..end].iter().map(|(_, c)| *c).collect();
        let name = name.trim().to_string();

        if name.len() < 2 {
            return None;
        }

        let type_hint = if has_camel || self.is_camelcase(&name) {
            "CamelCase"
        } else if name.chars().all(|c| Self::is_upper(c) || c.is_whitespace()) {
            "Acronym"
        } else {
            "ProperNoun"
        };

        Some((name, type_hint.to_string(), end))
    }

    fn scan_camel_word(&self, chars: &[(usize, char)], start: usize) -> usize {
        let len = chars.len();
        if start >= len || !Self::is_upper(chars[start].1) {
            return start;
        }
        let mut end = start + 1;
        while end < len {
            if Self::is_lower(chars[end].1) {
                end += 1;
            } else if Self::is_upper(chars[end].1) {
                if end + 1 < len && Self::is_lower(chars[end + 1].1) {
                    end += 1;
                } else if end + 1 < len && Self::is_upper(chars[end + 1].1) {
                    // ALL_CAPS suffix
                    end += 1;
                    while end < len && Self::is_upper(chars[end].1) {
                        end += 1;
                    }
                    if end < len && Self::is_lower(chars[end].1) {
                        end += 1;
                        while end < len && Self::is_lower(chars[end].1) {
                            end += 1;
                        }
                    }
                    break;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        end
    }

    fn find_word_end(&self, chars: &[(usize, char)], start: usize) -> usize {
        let len = chars.len();
        let mut end = start;
        while end < len && Self::is_word_char(chars[end].1) {
            end += 1;
        }
        end
    }

    fn scan_all_caps(&self, chars: &[(usize, char)], start: usize) -> Option<(String, String, usize)> {
        let len = chars.len();
        let mut end = start;
        while end < len && Self::is_upper(chars[end].1) {
            end += 1;
        }
        let name: String = chars[start..end].iter().map(|(_, c)| *c).collect();
        if name.len() >= 2 {
            Some((name, "Acronym".to_string(), end))
        } else {
            None
        }
    }

    fn scan_quoted(&self, text: &str) -> Vec<(String, usize)> {
        let mut results = Vec::new();
        let mut in_quote = false;
        let mut quote_char = '"';
        let mut current = String::new();
        let mut start_pos = 0;

        for (i, c) in text.char_indices() {
            if !in_quote && (c == '"' || c == '\'') {
                in_quote = true;
                quote_char = c;
                current.clear();
                start_pos = i + 1;
            } else if in_quote && c == quote_char {
                if !current.is_empty() {
                    results.push((current.clone(), start_pos));
                }
                in_quote = false;
            } else if in_quote {
                current.push(c);
            }
        }
        results
    }

    /// Extract lowercase noun phrases, skipping positions already covered by entities.
    fn extract_lowercase_phrases(
        &self,
        text: &str,
        entities: &mut Vec<ExtractedEntity>,
        seen: &mut HashMap<String, usize>,
        covered: &[(usize, usize)],
    ) {
        let chars: Vec<(usize, char)> = text.char_indices().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            let (pos, c) = chars[i];

            // Skip if this position is already covered by an entity
            if covered.iter().any(|(s, e)| pos >= *s && pos < *e) {
                i += 1;
                continue;
            }

            if Self::is_lower(c) {
                // Check that this is the start of a word (preceded by space or start of text)
                if i > 0 {
                    let prev_char = chars[i - 1].1;
                    if Self::is_word_char(prev_char) {
                        i += 1;
                        continue;
                    }
                }

                // Skip if this word is a verb keyword (e.g., "causes", "contradicts")
                let word_start = i;
                let word_end = self.find_word_end(&chars, word_start);
                let first_word: String = chars[word_start..word_end].iter().map(|(_, ch)| *ch).collect();
                if self.is_verb_keyword(&first_word) {
                    i = word_end;
                    continue;
                }

                let start = i;
                let mut end = i + 1;
                while end < len {
                    if Self::is_lower(chars[end].1) || chars[end].1 == '-' || chars[end].1 == '_' {
                        end += 1;
                    } else if chars[end].1 == ' ' && end + 1 < len && Self::is_lower(chars[end + 1].1) {
                        // Check the next word isn't a common word or verb keyword
                        let next_word_start = end + 1;
                        let next_word_end = self.find_word_end(&chars, next_word_start);
                        let next_word: String = chars[next_word_start..next_word_end].iter().map(|(_, ch)| *ch).collect();
                        if self.is_common_word(&next_word) || self.is_verb_keyword(&next_word) {
                            break;
                        }
                        end += 1;
                    } else {
                        break;
                    }
                }

                let phrase: String = chars[start..end].iter().map(|(_, ch)| *ch).collect();
                let trimmed = phrase.trim().to_string();

                // Only keep phrases with at least 2 words, and first word must not be common
                let first_word = trimmed.split_whitespace().next().unwrap_or("");
                if trimmed.split_whitespace().count() >= 2
                    && trimmed.len() > 3
                    && !self.is_common_word(first_word)
                    && !self.is_verb_keyword(first_word)
                {
                    self.add_entity(entities, seen, trimmed, "Concept", 0.6, pos);
                }

                i = end;
            } else {
                i += 1;
            }
        }
    }

    // --- Helpers ---

    fn is_camelcase(&self, s: &str) -> bool {
        s.chars().any(|c| Self::is_upper(c)) && s.chars().any(|c| Self::is_lower(c))
            && !s.starts_with(|c: char| Self::is_lower(c))
    }

    fn is_common_word(&self, name: &str) -> bool {
        matches!(name,
            "The" | "This" | "That" | "These" | "Those" | "There" | "Their" | "Then" | "Thus" | "Hence"
            | "When" | "Where" | "What" | "Which" | "While" | "With" | "From" | "Into" | "Over" | "Under"
            | "About" | "After" | "Before" | "Between" | "Through" | "During" | "Without" | "Within"
            | "Along" | "Among" | "Around" | "Across" | "Against" | "Toward" | "Throughout" | "Towards"
            | "Despite" | "Besides" | "For" | "And" | "Nor" | "But" | "Or" | "Yet" | "So" | "If"
            | "Because" | "Although" | "Since" | "Unless" | "Until" | "Are" | "Is" | "Was" | "Were"
            | "Be" | "Been" | "Being" | "Have" | "Has" | "Had" | "Do" | "Does" | "Did" | "Will"
            | "Would" | "Could" | "Should" | "May" | "Might" | "Shall" | "Can" | "Need" | "Dare"
            | "Ought" | "Used" | "To" | "Of" | "In" | "On" | "At" | "By" | "As" | "An" | "A"
            | "Not" | "No" | "Than" | "Too" | "Very" | "Just" | "Also" | "Both" | "Each" | "Few"
            | "More" | "Most" | "Other" | "Some" | "Such" | "Only" | "Own" | "Same" | "All"
        )
    }

    fn is_verb_keyword(&self, word: &str) -> bool {
        matches!(word,
            "causes" | "contradicts" | "prevents" | "supports" | "enables" | "implies"
            | "triggers" | "activates" | "inhibits" | "reinforces" | "leads" | "blocks"
            | "opposes" | "results" | "derives" | "part" | "instance" | "type"
            | "is" | "are" | "has" | "have" | "was" | "were"
        )
    }

    fn classify_type(&self, _name: &str, hint: &str) -> String {
        match hint {
            "CamelCase" | "Acronym" => "Entity".to_string(),
            "ProperNoun" => "Concept".to_string(),
            _ => "Concept".to_string(),
        }
    }

    fn confidence_for(&self, hint: &str) -> f64 {
        match hint {
            "CamelCase" => 0.9,
            "Acronym" => 0.85,
            "ProperNoun" => 0.8,
            _ => 0.6,
        }
    }

    fn add_entity(
        &self,
        entities: &mut Vec<ExtractedEntity>,
        seen: &mut HashMap<String, usize>,
        name: String,
        entity_type: &str,
        confidence: f64,
        position: usize,
    ) {
        if self.deduplicate {
            let key = name.to_lowercase();
            if let Some(&idx) = seen.get(&key) {
                if confidence > entities[idx].confidence {
                    entities[idx] = ExtractedEntity {
                        name, entity_type: entity_type.to_string(), confidence, position,
                    };
                }
                return;
            }
            seen.insert(key, entities.len());
        }
        entities.push(ExtractedEntity {
            name, entity_type: entity_type.to_string(), confidence, position,
        });
    }

    fn dedup_relations(&self, relations: Vec<ExtractedRelation>) -> Vec<ExtractedRelation> {
        let mut best: HashMap<(usize, usize, String), ExtractedRelation> = HashMap::new();
        for rel in relations {
            let key = (rel.src_idx, rel.dst_idx, rel.relation_type.clone());
            match best.get(&key) {
                Some(e) if e.confidence >= rel.confidence => {}
                _ => { best.insert(key, rel); }
            }
        }
        let mut r: Vec<_> = best.into_values().collect();
        r.sort_by(|a, b| a.src_idx.cmp(&b.src_idx).then(a.dst_idx.cmp(&b.dst_idx)));
        r
    }
}

impl ExtractedEntity {
    fn end_position(&self) -> usize {
        self.position + self.name.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proper_nouns() {
        let eng = IngestEngine::new();
        let text = "Neural Networks are a subset of Machine Learning. Backpropagation is used for training.";
        let ents = eng.extract_entities(text);
        let names: Vec<&str> = ents.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"Neural Networks"), "got: {:?}", names);
        assert!(names.contains(&"Machine Learning"), "got: {:?}", names);
        assert!(names.contains(&"Backpropagation"), "got: {:?}", names);
    }

    #[test]
    fn test_camelcase() {
        let eng = IngestEngine::new();
        let text = "The ReLU activation function and BatchNormalization layer are used.";
        let ents = eng.extract_entities(text);
        let names: Vec<&str> = ents.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"ReLU"), "got: {:?}", names);
        assert!(names.contains(&"BatchNormalization"), "got: {:?}", names);
    }

    #[test]
    fn test_acronyms() {
        let eng = IngestEngine::new();
        let text = "CNN and RNN are neural network types. NLP uses transformers.";
        let ents = eng.extract_entities(text);
        let names: Vec<&str> = ents.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"CNN"), "got: {:?}", names);
        assert!(names.contains(&"RNN"), "got: {:?}", names);
        assert!(names.contains(&"NLP"), "got: {:?}", names);
    }

    #[test]
    fn test_quoted() {
        let eng = IngestEngine::new();
        let text = r#"The term 'attention mechanism' was introduced. "Self-attention" is key."#;
        let ents = eng.extract_entities(text);
        let names: Vec<&str> = ents.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"attention mechanism"), "got: {:?}", names);
        assert!(names.contains(&"Self-attention"), "got: {:?}", names);
    }

    #[test]
    fn test_dedup() {
        let eng = IngestEngine::new();
        let text = "Neural Networks are powerful. Neural Networks can learn.";
        let ents = eng.extract_entities(text);
        assert_eq!(ents.iter().filter(|e| e.name == "Neural Networks").count(), 1);
    }

    #[test]
    fn test_relation_instanceof() {
        let eng = IngestEngine::new();
        let text = "Neural Networks is a subset of Machine Learning.";
        let ents = eng.extract_entities(text);
        let rels = eng.extract_relations(&ents, text);
        assert!(rels.iter().any(|r| r.relation_type == "InstanceOf"),
            "entities: {:?}, relations: {:?}", ents, rels);
    }

    #[test]
    fn test_relation_causes() {
        let eng = IngestEngine::new();
        let text = "Overfitting causes poor generalization.";
        let ents = eng.extract_entities(text);
        let rels = eng.extract_relations(&ents, text);
        assert!(rels.iter().any(|r| r.relation_type == "Causes"),
            "entities: {:?}, relations: {:?}", ents, rels);
    }

    #[test]
    fn test_relation_contradicts() {
        let eng = IngestEngine::new();
        let text = "Supervised learning contradicts unsupervised learning.";
        let ents = eng.extract_entities(text);
        let rels = eng.extract_relations(&ents, text);
        assert!(rels.iter().any(|r| r.relation_type == "Contradicts"),
            "entities: {:?}, relations: {:?}", ents, rels);
    }

    #[test]
    fn test_cooccurrence() {
        let eng = IngestEngine::new();
        let text = "Backpropagation and gradient descent are optimization techniques.";
        let ents = eng.extract_entities(text);
        let rels = eng.extract_relations(&ents, text);
        assert!(rels.iter().any(|r| r.relation_type == "CoOccursWith"),
            "entities: {:?}, relations: {:?}", ents, rels);
    }

    #[test]
    fn test_empty() {
        let eng = IngestEngine::new();
        assert!(eng.extract_entities("").is_empty());
        assert!(eng.extract_relations(&[], "").is_empty());
    }

    #[test]
    fn test_full_pipeline() {
        let eng = IngestEngine::new();
        let text = "Neural Networks are a subset of Machine Learning. Backpropagation causes weight updates. Gradient Descent optimizes the loss function.";
        let ents = eng.extract_entities(text);
        let rels = eng.extract_relations(&ents, text);
        assert!(ents.len() >= 3, "entities: {:?}", ents);
        assert!(rels.len() >= 1, "relations: {:?}", rels);
    }
}
