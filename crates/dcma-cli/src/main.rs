use clap::{Parser, Subcommand};
use dcma_core::atom::CognitiveAtom;
use dcma_core::state::CognitiveState;
use dcma_core::types::*;
use dcma_runtime::{Runtime, Stimulus};
use dcma_storage::Storage;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "dcma", about = "DCMA Cognitive Engine CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(global = true, long, default_value = ".")]
    path: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new cognitive state
    Init,
    /// Insert a new atom
    Insert {
        #[arg(long)]
        r#type: String,
        #[arg(long)]
        name: String,
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
    /// Run a retrieval query
    Query {
        #[arg(long)]
        nodes: String,
    },
    /// Execute one cognitive tick
    Tick {
        #[arg(long)]
        sources: Option<String>,
    },
    /// Start the HTTP API server
    Serve {
        #[arg(long, default_value = "3000")]
        port: u16,
    },
    /// Show graph statistics
    Stats,
    /// Export state to file
    Export {
        #[arg(long)]
        format: String,
    },
    /// Import state from file
    Import {
        #[arg(long)]
        format: String,
        #[arg(long)]
        file: PathBuf,
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
        "derivesfrom" => Ok(RelationType::DerivesFrom),
        "partof" => Ok(RelationType::PartOf),
        "instanceof" => Ok(RelationType::InstanceOf),
        "specializes" => Ok(RelationType::Specializes),
        "generalizes" => Ok(RelationType::Generalizes),
        "contains" => Ok(RelationType::Contains),
        "abstractionof" => Ok(RelationType::AbstractionOf),
        "implies" => Ok(RelationType::Implies),
        "negates" => Ok(RelationType::Negates),
        "supports" => Ok(RelationType::Supports),
        "contradicts" => Ok(RelationType::Contradicts),
        "precedes" => Ok(RelationType::Precedes),
        "follows" => Ok(RelationType::Follows),
        "cooccuurswith" | "cooccurs_with" => Ok(RelationType::CoOccursWith),
        "persistswith" => Ok(RelationType::PersistsWith),
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
    path.join("dcma_state.bin")
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
        Commands::Insert { r#type, name } => {
            let atom_type = match parse_atom_type(&r#type) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return;
                }
            };
            let (mut rt, path) = load_or_create(&cli.path);
            let id = AtomId::new();
            let atom = CognitiveAtom::new(id, atom_type, name);
            rt.state_mut().graph.add_atom(atom);
            println!("Inserted atom: {}", id);
            save_state(&rt, &path);
        }
        Commands::Connect { src, dst, r#type } => {
            let src_id = match parse_atom_id(&src) {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return;
                }
            };
            let dst_id = match parse_atom_id(&dst) {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return;
                }
            };
            let rel_type = match parse_relation_type(&r#type) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return;
                }
            };
            let (mut rt, path) = load_or_create(&cli.path);
            match rt.state_mut().graph.add_relation(src_id, dst_id, rel_type) {
                Ok(_) => println!("Created relation"),
                Err(e) => eprintln!("Error: {}", e),
            }
            save_state(&rt, &path);
        }
        Commands::Query { nodes } => {
            let ids: Vec<AtomId> = nodes
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| parse_atom_id(s.trim()))
                .collect::<Result<Vec<_>, _>>()
                .unwrap_or_default();
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
                .map(|s| {
                    s.split(',')
                        .filter(|s| !s.is_empty())
                        .map(|s| parse_atom_id(s.trim()))
                        .collect::<Result<Vec<_>, _>>()
                        .unwrap_or_default()
                })
                .unwrap_or_default();
            let stimulus = Stimulus {
                source_nodes: source_ids,
                query_context: vec![],
                label: "cli".into(),
            };
            let result = rt.tick(&stimulus);
            println!(
                "Tick {} completed (converged: {})",
                result.tick, result.convergence.converged
            );
            save_state(&rt, &path);
        }
        Commands::Serve { port } => {
            use std::net::SocketAddr;
            let (rt, _) = load_or_create(&cli.path);
            let app = dcma_api::app(rt);
            let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
            println!("Starting DCMA API server on {}", addr);
            let tokio_rt = tokio::runtime::Runtime::new().unwrap();
            tokio_rt.block_on(async {
                let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
                axum::serve(listener, app.into_make_service()).await.unwrap();
            });
        }
        Commands::Stats => {
            let (rt, _) = load_or_create(&cli.path);
            let state = rt.state();
            println!("Graph statistics:");
            println!("  Atoms: {}", state.graph.atom_count());
            println!("  Relations: {}", state.graph.relation_count());
            println!("  Tick: {}", state.tick);
            println!("  Degree distribution: {:?}", state.graph.degree_distribution());
            println!("  Diameter: {:?}", state.graph.diameter());
        }
        Commands::Export { format } => {
            let (rt, _) = load_or_create(&cli.path);
            let state = rt.state();
            match format.to_lowercase().as_str() {
                "json" => match serde_json::to_string_pretty(state) {
                    Ok(json) => println!("{}", json),
                    Err(e) => eprintln!("Error: {}", e),
                },
                _ => eprintln!("Unknown format: {}. Use 'json'.", format),
            }
        }
        Commands::Import { format, file } => {
            let (mut rt, path) = load_or_create(&cli.path);
            match format.to_lowercase().as_str() {
                "json" => match Storage::load_state_json(&file) {
                    Ok(state) => {
                        *rt.state_mut() = state;
                        println!("Imported state from {}", file.display());
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        return;
                    }
                },
                _ => {
                    eprintln!("Unknown format: {}. Use 'json'.", format);
                    return;
                }
            }
            save_state(&rt, &path);
        }
    }
}
