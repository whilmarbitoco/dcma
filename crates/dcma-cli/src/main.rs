use clap::{Parser, Subcommand};
use dcma_core::atom::CognitiveAtom;
use dcma_core::state::CognitiveState;
use dcma_core::types::*;
use dcma_runtime::{Runtime, Stimulus};
use dcma_storage::Storage;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "dcma", about = "DCMA — Deterministic Cognitive Memory Architecture", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(global = true, long, default_value = ".")]
    path: PathBuf,

    #[arg(global = true, long, help = "Output as JSON")]
    json: bool,

    #[arg(global = true, long, help = "PostgreSQL connection URL (e.g. postgres://user:pass@host/db)")]
    pg_url: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new cognitive state
    Init,
    /// Insert a new atom
    Add {
        #[arg(long)]
        r#type: String,
        #[arg(long)]
        name: String,
    },
    /// Search atoms by name (substring match)
    Search {
        /// Query string
        query: String,
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// List all atoms (paginated)
    List {
        #[arg(long, default_value = "20")]
        limit: usize,
        #[arg(long, default_value = "0")]
        offset: usize,
    },
    /// Show details of a specific atom
    Get {
        /// Atom ID (UUID)
        id: String,
    },
    /// Delete an atom
    Remove {
        /// Atom ID (UUID)
        id: String,
    },
    /// Create a relation between two atoms
    Connect {
        #[arg(long)]
        src: String,
        #[arg(long)]
        dst: String,
        #[arg(long)]
        r#type: String,
    },
    /// Ingest natural language text — extract entities and add as atoms
    Ingest {
        /// Text to ingest
        text: String,
    },
    /// Run a retrieval query
    Query {
        #[arg(long, value_delimiter = ',')]
        nodes: Vec<String>,
    },
    /// Execute one cognitive tick
    Tick {
        #[arg(long, value_delimiter = ',')]
        sources: Option<String>,
    },
    /// Show graph statistics
    Stats,
    /// Export state to JSON
    Export {
        #[arg(long, default_value = "json")]
        format: String,
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Import state from JSON
    Import {
        /// File to import
        file: PathBuf,
    },
    /// Start the HTTP API server
    Serve {
        #[arg(long, default_value = "3000")]
        port: u16,
    },
}

fn parse_atom_id(s: &str) -> Result<AtomId, String> {
    let uuid = uuid::Uuid::parse_str(s).map_err(|e| format!("Invalid atom ID: {}", e))?;
    Ok(AtomId(uuid))
}

fn parse_relation_type(s: &str) -> Result<RelationType, String> {
    match s.to_lowercase().as_str() {
        "causes" => Ok(RelationType::Causes),
        "prevents" => Ok(RelationType::Prevents),
        "enables" => Ok(RelationType::Enables),
        "triggers" => Ok(RelationType::Triggers),
        "derivesfrom" | "derives_from" => Ok(RelationType::DerivesFrom),
        "partof" | "part_of" => Ok(RelationType::PartOf),
        "instanceof" | "instance_of" => Ok(RelationType::InstanceOf),
        "specializes" => Ok(RelationType::Specializes),
        "generalizes" => Ok(RelationType::Generalizes),
        "contains" => Ok(RelationType::Contains),
        "abstractionof" | "abstraction_of" => Ok(RelationType::AbstractionOf),
        "implies" => Ok(RelationType::Implies),
        "negates" => Ok(RelationType::Negates),
        "supports" => Ok(RelationType::Supports),
        "contradicts" => Ok(RelationType::Contradicts),
        "precedes" => Ok(RelationType::Precedes),
        "follows" => Ok(RelationType::Follows),
        "cooccurswith" | "co_occurs_with" | "cooccurs_with" => Ok(RelationType::CoOccursWith),
        "persistswith" | "persists_with" => Ok(RelationType::PersistsWith),
        "reinforces" => Ok(RelationType::Reinforces),
        "inhibits" => Ok(RelationType::Inhibits),
        "activates" => Ok(RelationType::Activates),
        "suppresses" => Ok(RelationType::Suppresses),
        "focuses" => Ok(RelationType::Focuses),
        _ => Err(format!("Unknown relation type: {}", s)),
    }
}

fn parse_atom_type(s: &str) -> Result<AtomType, String> {
    match s.to_lowercase().as_str() {
        "concept" => Ok(AtomType::Concept),
        "event" => Ok(AtomType::Event),
        "entity" => Ok(AtomType::Entity),
        "procedure" => Ok(AtomType::Procedure),
        "abstraction" => Ok(AtomType::Abstraction),
        _ => Err(format!("Unknown atom type: {}", s)),
    }
}

fn get_state_path(path: &PathBuf) -> PathBuf {
    path.join("dcma_state.json")
}

fn load_or_create(path: &PathBuf) -> (Runtime, PathBuf) {
    let state_path = get_state_path(path);
    let config = EngineConfig::default();
    if state_path.exists() {
        match Storage::load_state(&state_path) {
            Ok(state) => (Runtime::with_state(config, state), state_path),
            Err(e) => {
                eprintln!("Warning: failed to load state ({}), creating new", e);
                (Runtime::new(config), state_path)
            }
        }
    } else {
        (Runtime::new(config), state_path)
    }
}

/// Load state from PostgreSQL or create a new one
fn load_or_create_pg(url: &str) -> (dcma_runtime::PgRuntime, dcma_pg::PgStorage) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let storage = dcma_pg::PgStorage::connect(url).await
            .unwrap_or_else(|e| { eprintln!("Error connecting to PostgreSQL: {}", e); std::process::exit(1); });
        let config = EngineConfig::default();
        let state = storage.load_state().await.unwrap_or_else(|_| CognitiveState::new(config.clone()));
        let runtime = dcma_runtime::PgRuntime::with_state_and_storage(config, state, storage.clone());
        (runtime, storage)
    })
}

/// Persist state to PostgreSQL
fn save_pg(storage: &dcma_pg::PgStorage, state: &CognitiveState) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        if let Err(e) = storage.save_state(state).await {
            eprintln!("Error saving state to PostgreSQL: {}", e);
        }
    });
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            if let Some(ref url) = cli.pg_url {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    match dcma_pg::PgStorage::connect(url).await {
                        Ok(storage) => {
                            storage.migrate().await.unwrap_or_else(|e| eprintln!("Warning: migration: {}", e));
                            let state = CognitiveState::new(EngineConfig::default());
                            storage.save_state(&state).await.unwrap_or_else(|e| eprintln!("Error: {}", e));
                            println!("Initialized cognitive state in PostgreSQL");
                        }
                        Err(e) => eprintln!("Error connecting to PostgreSQL: {}", e),
                    }
                });
            } else {
                let state = CognitiveState::new(EngineConfig::default());
                let path = get_state_path(&cli.path);
                match Storage::save_state(&state, &path) {
                    Ok(_) => println!("Initialized cognitive state at {}", path.display()),
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
        }
        Commands::Add { r#type, name } => {
            let atom_type = match parse_atom_type(&r#type) {
                Ok(t) => t,
                Err(e) => { eprintln!("Error: {}", e); return; }
            };
            if let Some(ref url) = cli.pg_url {
                let mut rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let storage = dcma_pg::PgStorage::connect(url).await.unwrap_or_else(|e| { eprintln!("Error: {}", e); std::process::exit(1); });
                    let mut state = storage.load_state().await.unwrap_or_else(|_| CognitiveState::new(EngineConfig::default()));
                    let id = AtomId::new();
                    let atom = CognitiveAtom::new(id, atom_type, name.clone());
                    state.graph.add_atom(atom);
                    if let Err(e) = storage.save_state(&state).await {
                        eprintln!("Error: {}", e);
                    } else {
                        println!("Added atom [{}] {} ({:?})", id, name, atom_type);
                    }
                });
            } else {
                let (mut rt, path) = load_or_create(&cli.path);
                let id = AtomId::new();
                let atom = CognitiveAtom::new(id, atom_type, name.clone());
                rt.state_mut().graph.add_atom(atom);
                println!("Added atom [{}] {} ({:?})", id, name, atom_type);
                if let Err(e) = Storage::save_state(rt.state(), &path) {
                    eprintln!("Error saving state: {}", e);
                }
            }
        }
        Commands::Search { query, limit } => {
            let q_lower = query.to_lowercase();
            let results = if let Some(ref url) = cli.pg_url {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let storage = dcma_pg::PgStorage::connect(url).await.unwrap_or_else(|e| { eprintln!("Error: {}", e); std::process::exit(1); });
                    let ids = storage.search_atoms(&query, limit as i64).await.unwrap_or_default();
                    let state = storage.load_state().await.unwrap_or_else(|_| CognitiveState::new(EngineConfig::default()));
                    let mut results = Vec::new();
                    for id in &ids {
                        if let Some(atom) = state.graph.get_atom(*id) {
                            let activation = state.get_activation(atom.id);
                            results.push(serde_json::json!({
                                "id": atom.id.to_string(),
                                "name": atom.canonical,
                                "type": format!("{:?}", atom.atom_type),
                                "state": format!("{:?}", atom.state),
                                "activation": activation.map(|a| a.get().to_string()),
                                "salience": atom.salience.get().to_string(),
                                "confidence": atom.confidence.get().to_string(),
                            }));
                        }
                    }
                    results
                })
            } else {
                let (rt, _) = load_or_create(&cli.path);
                let state = rt.state();
                state.graph.atoms()
                    .filter_map(|(_, atom)| {
                        if atom.canonical.to_lowercase().contains(&q_lower) {
                            let activation = state.get_activation(atom.id);
                            Some(serde_json::json!({
                                "id": atom.id.to_string(),
                                "name": atom.canonical,
                                "type": format!("{:?}", atom.atom_type),
                                "state": format!("{:?}", atom.state),
                                "activation": activation.map(|a| a.get().to_string()),
                                "salience": atom.salience.get().to_string(),
                                "confidence": atom.confidence.get().to_string(),
                            }))
                        } else {
                            None
                        }
                    })
                    .take(limit)
                    .collect()
            };

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&results).unwrap());
            } else {
                println!("Found {} atoms matching '{}':", results.len(), query);
                for atom in &results {
                    println!("  {}  {}  [{:?}]",
                        atom["id"].as_str().unwrap(),
                        atom["name"].as_str().unwrap(),
                        atom["type"].as_str().unwrap()
                    );
                }
            }
        }
        Commands::List { limit, offset } => {
            let (state, _was_pg) = if let Some(ref url) = cli.pg_url {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let storage = dcma_pg::PgStorage::connect(url).await.unwrap_or_else(|e| { eprintln!("Error: {}", e); std::process::exit(1); });
                    (storage.load_state().await.unwrap_or_else(|_| CognitiveState::new(EngineConfig::default())), true)
                })
            } else {
                let (rt, _) = load_or_create(&cli.path);
                let state = rt.state().clone();
                (state, false)
            };

            let all: Vec<_> = state.graph.atoms()
                .filter_map(|(_, atom)| {
                    let activation = state.get_activation(atom.id);
                    Some(serde_json::json!({
                        "id": atom.id.to_string(),
                        "name": atom.canonical,
                        "type": format!("{:?}", atom.atom_type),
                        "state": format!("{:?}", atom.state),
                        "activation": activation.map(|a| a.get().to_string()),
                    }))
                })
                .collect();

            let total = all.len();
            let page: Vec<_> = all.into_iter().skip(offset).take(limit).collect();

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({ "total": total, "offset": offset, "limit": limit, "atoms": page })).unwrap());
            } else {
                println!("Showing {}–{} of {} atoms:", offset + 1, offset + page.len(), total);
                for atom in &page {
                    println!("  {}  {}  [{:?}]",
                        atom["id"].as_str().unwrap(),
                        atom["name"].as_str().unwrap(),
                        atom["type"].as_str().unwrap()
                    );
                }
            }
        }
        Commands::Get { id } => {
            let atom_id = match parse_atom_id(&id) {
                Ok(i) => i,
                Err(e) => { eprintln!("Error: {}", e); return; }
            };
            let (state, _was_pg) = if let Some(ref url) = cli.pg_url {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let storage = dcma_pg::PgStorage::connect(url).await.unwrap_or_else(|e| { eprintln!("Error: {}", e); std::process::exit(1); });
                    (storage.load_state().await.unwrap_or_else(|_| CognitiveState::new(EngineConfig::default())), true)
                })
            } else {
                let (rt, _) = load_or_create(&cli.path);
                (rt.state().clone(), false)
            };

            match state.graph.get_atom(atom_id) {
                Some(atom) => {
                    let activation = state.get_activation(atom.id);
                    if cli.json {
                        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                            "id": atom.id.to_string(),
                            "name": atom.canonical,
                            "type": format!("{:?}", atom.atom_type),
                            "state": format!("{:?}", atom.state),
                            "activation": activation.map(|a| a.get().to_string()),
                            "salience": atom.salience.get().to_string(),
                            "confidence": atom.confidence.get().to_string(),
                            "attributes": atom.attributes,
                        })).unwrap());
                    } else {
                        println!("Atom: {}", atom.id);
                        println!("  Name:       {}", atom.canonical);
                        println!("  Type:       {:?}", atom.atom_type);
                        println!("  State:      {:?}", atom.state);
                        println!("  Activation: {:?}", activation.map(|a| a.get().to_string()));
                        println!("  Salience:   {}", atom.salience.get());
                        println!("  Confidence: {}", atom.confidence.get());
                        if !atom.attributes.is_empty() {
                            println!("  Attributes: {:?}", atom.attributes);
                        }
                    }
                }
                None => {
                    eprintln!("Atom {} not found", id);
                }
            }
        }
        Commands::Remove { id } => {
            let atom_id = match parse_atom_id(&id) {
                Ok(i) => i,
                Err(e) => { eprintln!("Error: {}", e); return; }
            };
            if let Some(ref url) = cli.pg_url {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let storage = dcma_pg::PgStorage::connect(url).await.unwrap_or_else(|e| { eprintln!("Error: {}", e); std::process::exit(1); });
                    let mut state = storage.load_state().await.unwrap_or_else(|_| CognitiveState::new(EngineConfig::default()));
                    if state.graph.remove_atom(atom_id).is_some() {
                        if let Err(e) = storage.save_state(&state).await {
                            eprintln!("Error: {}", e);
                        } else {
                            println!("Removed atom {}", id);
                        }
                    } else {
                        eprintln!("Atom {} not found", id);
                    }
                });
            } else {
                let (mut rt, path) = load_or_create(&cli.path);
                match rt.state_mut().graph.remove_atom(atom_id) {
                    Some(_) => {
                        println!("Removed atom {}", id);
                        if let Err(e) = Storage::save_state(rt.state(), &path) {
                            eprintln!("Error saving state: {}", e);
                        }
                    }
                    None => eprintln!("Atom {} not found", id),
                }
            }
        }
        Commands::Connect { src, dst, r#type } => {
            let src_id = match parse_atom_id(&src) {
                Ok(i) => i,
                Err(e) => { eprintln!("Error: {}", e); return; }
            };
            let dst_id = match parse_atom_id(&dst) {
                Ok(i) => i,
                Err(e) => { eprintln!("Error: {}", e); return; }
            };
            let rel_type = match parse_relation_type(&r#type) {
                Ok(t) => t,
                Err(e) => { eprintln!("Error: {}", e); return; }
            };

            if let Some(ref url) = cli.pg_url {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let storage = dcma_pg::PgStorage::connect(url).await.unwrap_or_else(|e| { eprintln!("Error: {}", e); std::process::exit(1); });
                    let mut state = storage.load_state().await.unwrap_or_else(|_| CognitiveState::new(EngineConfig::default()));
                    match state.graph.add_relation(src_id, dst_id, rel_type) {
                        Ok(_) => {
                            if let Err(e) = storage.save_state(&state).await {
                                eprintln!("Error: {}", e);
                            } else {
                                println!("Created relation: {} --[{:?}]--> {}", src, rel_type, dst);
                            }
                        }
                        Err(e) => eprintln!("Error: {:?}", e),
                    }
                });
            } else {
                let (mut rt, path) = load_or_create(&cli.path);
                match rt.state_mut().graph.add_relation(src_id, dst_id, rel_type) {
                    Ok(_) => println!("Created relation: {} --[{:?}]--> {}", src, rel_type, dst),
                    Err(e) => eprintln!("Error: {:?}", e),
                }
                if let Err(e) = Storage::save_state(rt.state(), &path) {
                    eprintln!("Error saving state: {}", e);
                }
            }
        }
        Commands::Ingest { text } => {
            if let Some(ref url) = cli.pg_url {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let storage = dcma_pg::PgStorage::connect(url).await.unwrap_or_else(|e| { eprintln!("Error: {}", e); std::process::exit(1); });
                    let mut state = storage.load_state().await.unwrap_or_else(|_| CognitiveState::new(EngineConfig::default()));

                    let engine = dcma_ingest::IngestEngine::new();
                    let extracted = engine.extract_entities(&text);
                    let relations = engine.extract_relations(&extracted, &text);

                    let mut ids = Vec::new();
                    for entity in &extracted {
                        let atom_type = match entity.entity_type.as_str() {
                            "Entity" => AtomType::Entity,
                            _ => AtomType::Concept,
                        };
                        let id = AtomId::new();
                        let atom = CognitiveAtom::new(id, atom_type, entity.name.clone());
                        state.graph.add_atom(atom);
                        ids.push(id);
                    }

                    if let Err(e) = storage.save_state(&state).await {
                        eprintln!("Error: {}", e);
                    }

                    println!("Ingested {} entities and {} relations from text:", extracted.len(), relations.len());
                    for (i, entity) in extracted.iter().enumerate() {
                        println!("  [{}] {} ({})", ids[i].0, entity.name, entity.entity_type);
                    }
                    if !relations.is_empty() {
                        println!("  Relations found:");
                        for rel in &relations {
                            if let (Some(src), Some(dst)) = (extracted.get(rel.src_idx), extracted.get(rel.dst_idx)) {
                                println!("    {} --{}--> {}", src.name, rel.relation_type, dst.name);
                            }
                        }
                    }
                });
            } else {
                let (mut rt, path) = load_or_create(&cli.path);

                let engine = dcma_ingest::IngestEngine::new();
                let extracted = engine.extract_entities(&text);
                let relations = engine.extract_relations(&extracted, &text);

                let mut ids = Vec::new();
                for entity in &extracted {
                    let atom_type = match entity.entity_type.as_str() {
                        "Entity" => AtomType::Entity,
                        _ => AtomType::Concept,
                    };
                    let id = AtomId::new();
                    let atom = CognitiveAtom::new(id, atom_type, entity.name.clone());
                    rt.state_mut().graph.add_atom(atom);
                    ids.push(id);
                }

                println!("Ingested {} entities and {} relations from text:", extracted.len(), relations.len());
                for (i, entity) in extracted.iter().enumerate() {
                    println!("  [{}] {} ({})", ids[i].0, entity.name, entity.entity_type);
                }
                if !relations.is_empty() {
                    println!("  Relations found:");
                    for rel in &relations {
                        if let (Some(src), Some(dst)) = (extracted.get(rel.src_idx), extracted.get(rel.dst_idx)) {
                            println!("    {} --{}--> {}", src.name, rel.relation_type, dst.name);
                        }
                    }
                }
                if let Err(e) = Storage::save_state(rt.state(), &path) {
                    eprintln!("Error saving state: {}", e);
                }
            }
        }
        Commands::Query { nodes } => {
            let ids: Vec<AtomId> = nodes.iter()
                .filter(|s| !s.is_empty())
                .filter_map(|s| parse_atom_id(s).ok())
                .collect();
            let mut state = if let Some(ref url) = cli.pg_url {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let storage = dcma_pg::PgStorage::connect(url).await.unwrap_or_else(|e| { eprintln!("Error: {}", e); std::process::exit(1); });
                    storage.load_state().await.unwrap_or_else(|_| CognitiveState::new(EngineConfig::default()))
                })
            } else {
                let (rt, _) = load_or_create(&cli.path);
                let state_ref = rt.state();
                CognitiveState {
                    graph: state_ref.graph.clone(),
                    activations: state_ref.activations.clone(),
                    contradictions: state_ref.contradictions.clone(),
                    tick: state_ref.tick,
                    history: state_ref.history.clone(),
                    config: state_ref.config.clone(),
                }
            };

            let activations: HashMap<AtomId, Activation> =
                state.activation_values().into_iter().collect();
            let field = dcma_retrieval::retrieve(
                &state.graph,
                &ids,
                &activations,
                state.tick,
                &state.config,
            );
            // Reward evidence for atoms that appeared in retrieval results
            field.record_retrieval_evidence(&mut state.graph, state.tick);
            println!("Query result:");
            println!("  Nodes: {:?}", field.nodes);
            println!("  Relevance: {}", field.relevance_score);
        }
        Commands::Tick { sources } => {
            let source_ids: Vec<AtomId> = sources
                .map(|s| s.split(',').filter(|s| !s.is_empty()).filter_map(|s| parse_atom_id(s.trim()).ok()).collect())
                .unwrap_or_default();
            let stimulus = Stimulus {
                source_nodes: source_ids,
                query_context: vec![],
                label: "cli".into(),
            };

            if let Some(ref url) = cli.pg_url {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let storage = dcma_pg::PgStorage::connect(url).await.unwrap_or_else(|e| { eprintln!("Error: {}", e); std::process::exit(1); });
                    let config = EngineConfig::default();
                    let state = storage.load_state().await.unwrap_or_else(|_| CognitiveState::new(config.clone()));
                    let mut pg_runtime = dcma_runtime::PgRuntime::with_state_and_storage(config, state, storage);
                    let result = pg_runtime.tick(&stimulus).await;
                    println!("Tick {} completed (converged: {})", result.tick, result.convergence.converged);
                });
            } else {
                let (mut rt, path) = load_or_create(&cli.path);
                let result = rt.tick(&stimulus);
                println!("Tick {} completed (converged: {})", result.tick, result.convergence.converged);
                if let Err(e) = Storage::save_state(rt.state(), &path) {
                    eprintln!("Error saving state: {}", e);
                }
            }
        }
        Commands::Stats => {
            let state = if let Some(ref url) = cli.pg_url {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let storage = dcma_pg::PgStorage::connect(url).await.unwrap_or_else(|e| { eprintln!("Error: {}", e); std::process::exit(1); });
                    storage.load_state().await.unwrap_or_else(|_| CognitiveState::new(EngineConfig::default()))
                })
            } else {
                let (rt, _) = load_or_create(&cli.path);
                let state_ref = rt.state();
                CognitiveState {
                    graph: state_ref.graph.clone(),
                    activations: state_ref.activations.clone(),
                    contradictions: state_ref.contradictions.clone(),
                    tick: state_ref.tick,
                    history: state_ref.history.clone(),
                    config: state_ref.config.clone(),
                }
            };

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                    "atoms": state.graph.atom_count(),
                    "relations": state.graph.relation_count(),
                    "tick": state.tick,
                    "degree_distribution": state.graph.degree_distribution(),
                    "diameter": state.graph.diameter(),
                })).unwrap());
            } else {
                println!("Graph Statistics:");
                println!("  Atoms:      {}", state.graph.atom_count());
                println!("  Relations:  {}", state.graph.relation_count());
                println!("  Tick:       {}", state.tick);
                println!("  Diameter:   {:?}", state.graph.diameter());
                println!("  Degrees:    {:?}", state.graph.degree_distribution());
            }
        }
        Commands::Export { format, file } => {
            let state = if let Some(ref url) = cli.pg_url {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let storage = dcma_pg::PgStorage::connect(url).await.unwrap_or_else(|e| { eprintln!("Error: {}", e); std::process::exit(1); });
                    storage.load_state().await.unwrap_or_else(|_| CognitiveState::new(EngineConfig::default()))
                })
            } else {
                let (rt, _) = load_or_create(&cli.path);
                let state_ref = rt.state();
                CognitiveState {
                    graph: state_ref.graph.clone(),
                    activations: state_ref.activations.clone(),
                    contradictions: state_ref.contradictions.clone(),
                    tick: state_ref.tick,
                    history: state_ref.history.clone(),
                    config: state_ref.config.clone(),
                }
            };

            match format.to_lowercase().as_str() {
                "json" => {
                    let json = serde_json::to_string_pretty(&state).unwrap();
                    if let Some(f) = file {
                        std::fs::write(&f, &json).unwrap();
                        println!("Exported state to {}", f.display());
                    } else {
                        println!("{}", json);
                    }
                }
                _ => eprintln!("Unknown format: {}. Use 'json'.", format),
            }
        }
        Commands::Import { file } => {
            match &cli.pg_url {
                Some(url) => {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        let storage = dcma_pg::PgStorage::connect(url).await.unwrap_or_else(|e| { eprintln!("Error: {}", e); std::process::exit(1); });
                        match Storage::load_state(&file) {
                            Ok(state) => {
                                storage.save_state(&state).await.unwrap_or_else(|e| eprintln!("Error: {}", e));
                                println!("Imported state from {} to PostgreSQL", file.display());
                            }
                            Err(e) => eprintln!("Error loading file: {}", e),
                        }
                    });
                }
                None => {
                    let (mut rt, path) = load_or_create(&cli.path);
                    match Storage::load_state(&file) {
                        Ok(state) => {
                            *rt.state_mut() = state;
                            println!("Imported state from {}", file.display());
                            if let Err(e) = Storage::save_state(rt.state(), &path) {
                                eprintln!("Error saving state: {}", e);
                            }
                        }
                        Err(e) => eprintln!("Error: {}", e),
                    }
                }
            }
        }
        Commands::Serve { port } => {
            use std::net::SocketAddr;
            match &cli.pg_url {
                Some(url) => {
                    use axum::{
                        extract::{Path, State},
                        http::StatusCode,
                        routing::{get, post, delete},
                        Json, Router,
                    };
                    use std::sync::Arc;
                    use tokio::sync::Mutex;

                    type SharedPgRuntime = Arc<Mutex<dcma_runtime::PgRuntime>>;

                    let url_clone = url.clone();
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        let storage = dcma_pg::PgStorage::connect(&url_clone).await.unwrap_or_else(|e| { eprintln!("Error: {}", e); std::process::exit(1); });
                        let config = EngineConfig::default();
                        let state = storage.load_state().await.unwrap_or_else(|_| CognitiveState::new(config.clone()));
                        let pg_rt = dcma_runtime::PgRuntime::with_state_and_storage(config, state, storage);
                        let shared: SharedPgRuntime = Arc::new(Mutex::new(pg_rt));

                        let app = Router::new()
                            .route("/health", get({
                                let shared = shared.clone();
                                move || async move {
                                    let rt = shared.lock().await;
                                    Json(serde_json::json!({"status": "ok", "version": "0.1.0", "atoms": rt.state().graph.atom_count(), "relations": rt.state().graph.relation_count(), "tick": rt.tick_count()}))
                                }
                            }))
                            .route("/state", get({
                                let shared = shared.clone();
                                move || async move {
                                    let rt = shared.lock().await;
                                    Json(serde_json::json!({"nodes": rt.state().graph.atom_count(), "edges": rt.state().graph.relation_count(), "tick": rt.tick_count()}))
                                }
                            }))
                            .route("/tick", post({
                                let shared = shared.clone();
                                move |Json(body): Json<serde_json::Value>| async move {
                                    let mut rt = shared.lock().await;
                                    let sources: Vec<AtomId> = serde_json::from_value(body.get("sources").cloned().unwrap_or(serde_json::json!([]))).unwrap_or_default();
                                    let stimulus = Stimulus { source_nodes: sources, query_context: vec![], label: "api".into() };
                                    let result = rt.tick(&stimulus).await;
                                    Json(serde_json::json!({"tick": result.tick, "converged": result.convergence.converged}))
                                }
                            }))
                            .route("/atoms", get({
                                let shared = shared.clone();
                                move || async move {
                                    let rt = shared.lock().await;
                                    let atoms: Vec<_> = rt.state().graph.atoms().map(|(_, a)| serde_json::json!({"id": a.id.to_string(), "name": a.canonical, "type": format!("{:?}", a.atom_type)})).collect();
                                    Json(atoms)
                                }
                            }).post({
                                let shared = shared.clone();
                                move |Json(body): Json<serde_json::Value>| async move {
                                    let mut rt = shared.lock().await;
                                    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let atom_type = match body.get("type").and_then(|v| v.as_str()) {
                                        Some("Entity") => AtomType::Entity,
                                        Some("Event") => AtomType::Event,
                                        Some("Procedure") => AtomType::Procedure,
                                        Some("Abstraction") => AtomType::Abstraction,
                                        _ => AtomType::Concept,
                                    };
                                    let id = AtomId::new();
                                    let atom = CognitiveAtom::new(id, atom_type, name);
                                    rt.state_mut().graph.add_atom(atom);
                                    // Auto-persist handled by PgRuntime
                                    Json(serde_json::json!({"id": id.to_string()}))
                                }
                            }))
                            .route("/atoms/{id}", get({
                                let shared = shared.clone();
                                move |Path(id): Path<String>| async move {
                                    let rt = shared.lock().await;
                                    let atom_id = uuid::Uuid::parse_str(&id).ok().map(AtomId);
                                    if let Some(aid) = atom_id {
                                        if let Some(atom) = rt.state().graph.get_atom(aid) {
                                            return Json(serde_json::json!({"id": atom.id.to_string(), "name": atom.canonical, "type": format!("{:?}", atom.atom_type)}));
                                        }
                                    }
                                    Json(serde_json::json!({"error": "not found"}))
                                }
                            }).delete({
                                let shared = shared.clone();
                                move |Path(id): Path<String>| async move {
                                    let mut rt = shared.lock().await;
                                    let atom_id = uuid::Uuid::parse_str(&id).ok().map(AtomId);
                                    if let Some(aid) = atom_id {
                                        rt.state_mut().graph.remove_atom(aid);
                                    }
                                    Json(serde_json::json!({"status": "deleted"}))
                                }
                            }))
                            .route("/ingest", post({
                                let shared = shared.clone();
                                move |Json(body): Json<serde_json::Value>| async move {
                                    let text = body.get("text").and_then(|v| v.as_str()).unwrap_or("");
                                    let engine = dcma_ingest::IngestEngine::new();
                                    let extracted = engine.extract_entities(text);
                                    let mut rt = shared.lock().await;
                                    let mut count = 0;
                                    for entity in &extracted {
                                        let atom_type = match entity.entity_type.as_str() { "Entity" => AtomType::Entity, _ => AtomType::Concept };
                                        let id = AtomId::new();
                                        rt.state_mut().graph.add_atom(CognitiveAtom::new(id, atom_type, entity.name.clone()));
                                        count += 1;
                                    }
                                    Json(serde_json::json!({"atoms_created": count}))
                                }
                            }));

                        let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
                        println!("Starting DCMA API server on http://{} (PostgreSQL backend)", addr);
                        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
                        axum::serve(listener, app.into_make_service()).await.unwrap();
                    });
                }
                None => {
                    let (rt, _) = load_or_create(&cli.path);
                    let app = dcma_api::app(rt);
                    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
                    println!("Starting DCMA API server on http://{} (file backend)", addr);
                    let tokio_rt = tokio::runtime::Runtime::new().unwrap();
                    tokio_rt.block_on(async {
                        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
                        axum::serve(listener, app.into_make_service()).await.unwrap();
                    });
                }
            }
        }
    }
}
