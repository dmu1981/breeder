use clap::{Parser, Subcommand};
use genetics::*;
use ndarray::*;
use rand_distr::Normal;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

//----------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
struct MyPayload {
    botnet: BotNet,
    experiment: Uuid,
}

/*fn rand(min: u32, max: u32) -> u32 {
    min + rand::random::<u32>() % (max - min)
}*/

async fn spawn_new_genes(genepool: &mut GenePool<MyPayload>) -> Result<(), GenomeError> {
    let experiment = Uuid::new_v4();
    println!("Experiment UUID is {}", experiment);

    for _ in 0..genepool.population_size {
        genepool
            .add_genome(Genome::new(
                0,
                MyPayload {
                    botnet: BotNet::new(7, 50, 4),
                    experiment,
                },
            ))
            .await?;
    }

    Ok(())
}

fn breed_next_generation(
    genes: &[Genome<MyPayload>],
) -> Result<Vec<Genome<MyPayload>>, GenomeError> {
    println!("Breeding next generation!");

    let mut new_genes = Vec::<Genome<MyPayload>>::new();

    let mut total_fitness: f32 = 0.0;

    // Only preserve the 10 fittest individuals
    for x in &genes[0..40] {
        total_fitness += x.message.fitness.unwrap();
        println!(
            "Genome {} has fitness {}",
            x.message.uuid,
            x.message.fitness.unwrap()
        );

        // Keep this network as it is
        new_genes.push(Genome::new(
            x.message.generation + 1,
            MyPayload {
                botnet: x.message.payload.botnet.clone(),
                experiment: x.message.payload.experiment,
            },
        ));

        // Create 10 variants of it
        for variant in 0..14 {
            let dist = Normal::new(0.0, 0.05 + (variant as f32) * 0.005).unwrap();
            new_genes.push(Genome::new(
                x.message.generation + 1,
                MyPayload {
                    botnet: x.message.payload.botnet.variant(&dist),
                    experiment: x.message.payload.experiment,
                },
            ));
        }
    }
    println!("Total fitness was {}", total_fitness);

    Ok(new_genes)
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "amqp://guest:guest@127.0.0.1:5672")]
    pool: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Resets the queue and spawns new genes of generation 0
    Reset,
    /// Runs the monitor and breeds new genes if current generation is complete
    Run,
    /// Dumps all queues to disk without acknowleding them (can continue training afters)
    Dump,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    println!("Connecting to pool at {}", cli.pool);

    let v: f32 = 250.0 + rand::random::<f32>() * 1750.0;
    tokio::time::sleep(Duration::from_millis(v as u64)).await;

    let population_size = 600;

    match cli.command {
        Commands::Dump => {
            let mut genepool = GenePool::<MyPayload>::new(
                population_size,
                FitnessSortingOrder::LessIsBetter,
                cli.pool,
            )
            .unwrap();
            genepool.dump().unwrap();
        }
        Commands::Reset => {
            //println!("RESET DISABLED FOR NOW!");

            let mut genepool = GenePool::<MyPayload>::new(
                population_size,
                FitnessSortingOrder::LessIsBetter,
                cli.pool,
            )
            .unwrap();
            genepool.empty_pool().unwrap();
            spawn_new_genes(&mut genepool).await.unwrap();
        }
        Commands::Run => {
            let mut handles: Vec<tokio::task::JoinHandle<Result<(), GenomeError>>> = vec![];

            handles.push(tokio::spawn(async move {
                //let population_handler = MyPopulationHandler::new();
                let mut genepool = GenePool::<MyPayload>::new(
                    population_size,
                    FitnessSortingOrder::LessIsBetter,
                    cli.pool,
                )
                .unwrap();

                genepool.monitor(breed_next_generation).await?;

                Ok(())
            }));

            futures::future::join_all(handles).await;
        }
    }
}
