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

fn save_state(rt: &Runtime, path: &PathBuf) {
    if let Err(e) = Storage::save_state(rt.state(), path) {
        eprintln!("Error saving state: {}", e);
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            let state = CognitiveState::new(EngineConfig::default());
            let path = get_state_path(&cli.path);
            match Storage::save_state(&state, &path) {
                Ok(_) => println!("Initialized cognitive state at {}", path.display()),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::Add { r#type, name } => {
            let atom_type = match parse_atom_type(&r#type) {
                Ok(t) => t,
                Err(e) => { eprintln!("Error: {}", e); return; }
            };
            let (mut rt, path) = load_or_create(&cli.path);
            let id = AtomId::new();
            let atom = CognitiveAtom::new(id, atom_type, name.clone());
            rt.state_mut().graph.add_atom(atom);
            println!("Added atom [{}] {} ({:?})", id, name, atom_type);
            save_state(&rt, &path);
        }
        Commands::Search { query, limit } => {
            let (rt, _) = load_or_create(&cli.path);
            let state = rt.state();
            let q_lower = query.to_lowercase();

            let results: Vec<_> = state.graph.atoms()
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
                .collect();

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
            let (rt, _) = load_or_create(&cli.path);
            let state = rt.state();

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
            let (rt, _) = load_or_create(&cli.path);
            let state = rt.state();
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
            let (mut rt, path) = load_or_create(&cli.path);
            match rt.state_mut().graph.remove_atom(atom_id) {
                Some(_) => {
                    println!("Removed atom {}", id);
                    save_state(&rt, &path);
                }
                None => eprintln!("Atom {} not found", id),
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
            let (mut rt, path) = load_or_create(&cli.path);
            match rt.state_mut().graph.add_relation(src_id, dst_id, rel_type) {
                Ok(_) => println!("Created relation: {} --[{:?}]--> {}", src, rel_type, dst),
                Err(e) => eprintln!("Error: {:?}", e),
            }
            save_state(&rt, &path);
        }
        Commands::Ingest { text } => {
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
            save_state(&rt, &path);
        }
        Commands::Query { nodes } => {
            let ids: Vec<AtomId> = nodes.iter()
                .filter(|s| !s.is_empty())
                .filter_map(|s| parse_atom_id(s).ok())
                .collect();
            let (rt, _) = load_or_create(&cli.path);
            let state = rt.state();
            let activations: HashMap<AtomId, Activation> =
                state.activation_values().into_iter().collect();
            let field = dcma_retrieval::retrieve(
                &state.graph,
                &ids,
                &activations,
                state.tick,
                &state.config,
            );
            println!("Query result:");
            println!("  Nodes: {:?}", field.nodes);
            println!("  Relevance: {}", field.relevance_score);
        }
        Commands::Tick { sources } => {
            let (mut rt, path) = load_or_create(&cli.path);
            let source_ids: Vec<AtomId> = sources
                .map(|s| s.split(',').filter(|s| !s.is_empty()).filter_map(|s| parse_atom_id(s.trim()).ok()).collect())
                .unwrap_or_default();
            let stimulus = Stimulus {
                source_nodes: source_ids,
                query_context: vec![],
                label: "cli".into(),
            };
            let result = rt.tick(&stimulus);
            println!("Tick {} completed (converged: {})", result.tick, result.convergence.converged);
            save_state(&rt, &path);
        }
        Commands::Stats => {
            let (rt, _) = load_or_create(&cli.path);
            let state = rt.state();
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
            let (rt, _) = load_or_create(&cli.path);
            let state = rt.state();
            match format.to_lowercase().as_str() {
                "json" => {
                    let json = serde_json::to_string_pretty(state).unwrap();
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
            let (mut rt, path) = load_or_create(&cli.path);
            match Storage::load_state(&file) {
                Ok(state) => {
                    *rt.state_mut() = state;
                    println!("Imported state from {}", file.display());
                    save_state(&rt, &path);
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Commands::Serve { port } => {
            use std::net::SocketAddr;
            let (rt, _) = load_or_create(&cli.path);
            let app = dcma_api::app(rt);
            let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
            println!("Starting DCMA API server on http://{}", addr);
            let tokio_rt = tokio::runtime::Runtime::new().unwrap();
            tokio_rt.block_on(async {
                let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
                axum::serve(listener, app.into_make_service()).await.unwrap();
            });
        }
    }
}
