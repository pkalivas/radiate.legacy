
extern crate rayon;

use std::sync::{Arc, RwLock};
use std::marker::Sync;
use std::fmt::Debug;
use std::cmp::PartialEq;
use rayon::prelude::*;
use super::{
    generation::{Generation, Container},
    genome::Genome,
    problem::Problem,
    environment::Envionment,
    genocide::Genocide,
    survival::{SurvivalCriteria, ParentalCriteria}
};



/// Keep track of the number of stagnant generations the population has had 
/// if it reaches the target_stagnation, the vec of Genocides will be applied
#[derive(Debug, Clone, Serialize, Deserialize)] 
struct Stagnant {
    target_stagnation: usize,
    current_stagnation: usize,
    previous_top_score: f32,
    cleaners: Vec<Genocide>
}


/// This is just to keep track of a few parameters for 
/// the population, this encapsulates a few arguments for speciation
/// in the algorithm, these are specific to genetic algorithms 
/// which implement speciation between members of the population.
/// It also leaves room for more parameters to be added in the future.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub inbreed_rate: f32,
    pub crossover_rate: f32,
    pub distance: f32,
    pub species_target: usize
}


/// Population is what facilitates the evolution from a 5000 ft view
/// keeping track of what the generation is doing, marking statistics
/// down from each one, and holding resource sensitive things like
/// datasets as well as making sure that the optimization is moving 
/// forward through the stats it keeps (stagnation)
pub struct Population<T, E, P>
    where
        T: Genome<T, E> + Send + Sync,
        E: Envionment + Sized + Send + Sync,
        P: Problem<T>
{
    size: i32,
    dynamic_distance: bool,
    debug_progress: bool,
    config: Config,
    curr_gen: Generation<T, E>,
    stagnation: Stagnant,
    solve: Arc<RwLock<P>>,
    environment: Arc<RwLock<E>>,
    survivor_criteria: SurvivalCriteria,
    parental_criteria: ParentalCriteria
}




/// implement the population
impl<T, E, P> Population<T, E, P>
    where
        T: Genome<T, E> + Send + Sync + Clone,
        E: Envionment + Sized + Send + Sync + Default,
        P: Problem<T>
{

    /// base population
    pub fn new() -> Self {   
        Population {
            // define the number of members to participate in evolution and be injected into the current generation
            size: 100,
            // determin if the species should be aiming for a specific number of species by adjusting the distance threshold
            dynamic_distance: false,
            // debug_progress is only used to print out some information from each generation
            // to the console during training to get a glimpse into what is going on
            debug_progress: false,
            // create a new config to help the speciation of the population
            config: Config::new(),
            // create a new empty generation to be passed down through the population 
            curr_gen: Generation::<T, E>::new(),
            // keep track of fitness score stagnation through the population
            stagnation: Stagnant::new(0, Vec::new()),
            // Arc<Problem> so the problem can be sent between threads safely without duplicating the problem, 
            // if the problem gets duplicated every time a supervised learning problem with a lot of data could take up a ton of memory
            solve: Arc::new(RwLock::new(P::empty())),
            // create a new solver settings that will hold the specific settings for the defined solver 
            // that will allow the structure to evolve through generations
            environment: Arc::new(RwLock::new(E::default())),
            // determine which genomes will live on and pass down to the next generation
            survivor_criteria: SurvivalCriteria::Fittest,
            // determine how to pick parents to reproduce
            parental_criteria: ParentalCriteria::BiasedRandom
        }
    }

    /// Get mutable slice of current generation members.
    pub fn members_mut(&mut self) -> &mut [Container<T, E>] {
        self.curr_gen.members_mut()
    }

    /// Get mutable member.
    pub fn member_mut(&mut self, idx: usize) -> Option<&mut Container<T, E>> {
        self.curr_gen.member_mut(idx)
    }

    /// Get immutable member.
    pub fn member(&self, idx: usize) -> Option<&Container<T, E>> {
        self.curr_gen.member(idx)
    }

    /// Each generation will be trained by a call to this function 
    /// resulting optimization of the current generation, up to a 
    /// crossover into the next generation which will be set to the 
    /// new current generation
    #[inline]
    pub fn train(&mut self) -> Option<(f32, T)>
        where 
            T: Genome<T, E> + Clone + Send + Sync + Debug + PartialEq,
            P: Send + Sync
    {
        // optimize the population 
        self.curr_gen.optimize(self.solve.clone());
        self.end_generation()
    }

    /// Handle end of generation calculations and create a new generation.
    /// Returns the top member and their score.
    pub fn end_generation(&mut self) -> Option<(f32, T)>
        where 
            T: Genome<T, E> + Clone + Send + Sync + Debug + PartialEq,
            P: Send + Sync
    {
        let top_member = self.curr_gen.best_member()?;
        // adjust the distance of the population if needed
        if self.dynamic_distance { self.adjust_distance(); }
        // speciate the generation into niches then see if the population is stagnant
        // if the population is stagnant, clean the population 
        self.curr_gen.speciate(self.config.distance, Arc::clone(&self.environment));
        self.manage_stagnation(top_member.0);
        // If debug is set to true, this is the place to show it before the new generation is 
        if self.debug_progress { self.show_progress(); }
        // create a new generation and return it
        self.curr_gen = self.curr_gen.create_next_generation(self.size, self.config.clone(), Arc::clone(&self.environment))?;
        // return the top member score and the member
        Some((top_member.0, (*top_member.1).clone()))
    }

    /// Check to see if the population is stagnant or not, if it is,
    /// then go ahead and clean the population 
    fn manage_stagnation(&mut self, curr_top_score: f32) {
        if self.stagnation.target_stagnation == self.stagnation.current_stagnation {
            for cleaner in self.stagnation.cleaners.iter() {
                cleaner.kill(&mut self.curr_gen);
            }
            self.stagnation.current_stagnation = 0;
        } else if curr_top_score == self.stagnation.previous_top_score {
            self.stagnation.current_stagnation += 1;
        } else {
            self.stagnation.current_stagnation = 0;
        }
        self.stagnation.previous_top_score = curr_top_score;
    }

    /// dynamically adjust the distance of a population
    fn adjust_distance(&mut self) {
        if self.curr_gen.species.len() < self.config.species_target {
            self.config.distance -= 0.1;
        } else if self.curr_gen.species.len() > self.config.species_target {
            self.config.distance += 0.1;
        }
        if self.config.distance < 0.2 {
            self.config.distance = 0.1;
        }
    }

    /// Run the population according to a user defined function, the inputs of which
    /// are a borrowed member which is the top member of the current generation, 
    /// the fitness of that member, and the current number of generations.
    /// This function will continue until this function returns a true value 
    pub fn run<F>(&mut self, runner: F) -> Result<(T, E), &'static str>
        where 
            F: Fn(&T, f32, i32) -> bool + Sized,
            T: Genome<T, E> + Clone + Send + Sync + Debug + PartialEq,
            P: Send + Sync,
            E: Clone
    {
        let mut index = 0;
        loop {
            match self.train() {
                Some(result) => {
                    let (fit, top) = result;
                    if runner(&top, fit, index) {
                        let solution = top.clone();
                        let env = (*self.environment.read().unwrap()).clone();
                        return Ok((solution, env));
                    }
                    index += 1;
                },
                None => return Err("Error Training")
            }
        }
    }

    /// if debug is set to true, this is what will print out 
    /// the training to the screen during optimization.
    fn show_progress(&self) {
        println!("\n");
        for i in self.curr_gen.species.iter() {
            i.read().unwrap().display_info();
        }
    }
    
    /////////////////////////////////////////////////////////////////////////////////////////////////////////
    /// configure all the settings for the population these all have default settings if they are not set ///
    /// by hand, however you might find those default settings do not satisfy the needs of your problem   ///
    /////////////////////////////////////////////////////////////////////////////////////////////////////////
    
    /// Set the beginning generation of the population by a generation object
    /// this can be done in three ways all listed below.
    /// 
    /// 1.) populate_gen - Create a generation object outsize of this scope and give it to the 
    ///                    population, return the population back to the caller
    /// 2.) populate_base - as long as the population size has already been set and the type T has
    ///                     implemented the base trait fn, this will generate a new base generation
    /// 3.) populate_vec - Give the population a vec of type T and generate a new generation from it 
    ///                    then return the population back to the caller
    /// 4.) populate_clone - Take a base type T and create a population that is made up 
    ///                      completely of clones of this type - they will all be the same 
    ///                      at least for the first generation, this is useful for algorithms like NEAT
    
    /// give the populate a direct generation object 
    pub fn populate_gen(mut self, gen: Generation<T, E>) -> Self {
        self.curr_gen = gen;
        self
    }
    
    /// populate the populate with the base implementation of the genome 
    pub fn populate_base(mut self) -> Self 
        where P: Send + Sync
    {
        self.curr_gen = Generation {
            members: (0..self.size)
                .into_par_iter()
                .map(|_| {
                    let mut lock_set = self.environment.write().unwrap();
                    Container {
                        member: Arc::new(RwLock::new(T::base(&mut lock_set))),
                        fitness_score: 0.0,
                        species: None
                    }    
                })
                .collect(),
            species: Vec::new(),
            survival_criteria: SurvivalCriteria::Fittest,
            parental_criteria: ParentalCriteria::BiasedRandom
        };
        self
    }
    
    /// given a vec of type T which implements Genome, populate the population
    pub fn populate_vec(mut self, vals: Vec<T>) -> Self {
        self.curr_gen = Generation {
            members: vals.into_iter()
                .map(|x| {
                    Container {
                        member: Arc::new(RwLock::new(x)),
                        fitness_score: 0.0,
                        species: None
                    }
                })
                .collect(),
            species: Vec::new(),
            survival_criteria: SurvivalCriteria::Fittest,
            parental_criteria: ParentalCriteria::BiasedRandom
        };
        self
    }
    
    /// Given one type T which is a genome, create a population with clones of the original
    pub fn populate_clone(mut self, original: T) -> Self 
        where T: Genome<T, E> + Clone 
    {
        self.curr_gen = Generation {
            members: (0..self.size as usize)
                .into_iter()
                .map(|_| {
                    Container {
                        member: Arc::new(RwLock::new(original.clone())),
                        fitness_score: 0.0,
                        species: None
                    }
                })
                .collect(),
            species: Vec::new(),
            survival_criteria: SurvivalCriteria::Fittest,
            parental_criteria: ParentalCriteria::BiasedRandom
        };
        self
    }

    /// Give solver settings to the population to evolve the structure defined
    pub fn constrain(mut self, environment: E) -> Self {
        self.environment = Arc::new(RwLock::new(environment));
        self
    }

    /// Set the size of the population, the population size
    /// will default to 100 if this isn't set which could be enough 
    /// depending on the problem being solved 
    pub fn size(mut self, size: i32) -> Self {
        self.size = size;
        self
    }

    /// Get the size of the population. 
    pub fn get_size(&self) -> i32 {
        self.size
    }

    /// set the dynamic distance bool
    pub fn dynamic_distance(mut self, opt: bool) -> Self {
        self.dynamic_distance = opt;
        self
    }

    /// set the stagnation number of the population
    pub fn stagnation(mut self, stag: usize, cleaner: Vec<Genocide>) -> Self {
        self.stagnation = Stagnant::new(stag, cleaner);
        self
    }
   
    /// Set a config object to the population, these are arguments related
    /// to evolution through speciation, so these are all speciation
    /// arguments
    pub fn configure(mut self, spec: Config) -> Self {
        self.config = spec;
        self
    }
    
    /// Impose a problem on the population, in other words, 
    /// give the population a problem to solve. This 
    /// will default to an empty problem, meaning the population
    /// will not solve anything if this isn't set. This is really
    /// the most important argument for the population
    pub fn impose(mut self, prob: P) -> Self {
        self.solve = Arc::new(RwLock::new(prob));
        self
    }
    
    /// debug determines what to display to the screen during evolution
    pub fn debug(mut self, d: bool) -> Self {
        self.debug_progress = d;
        self
    }

    /// give the population a survival criteria, if none is supplied then it
    /// defaults to the fittest genome from each species
    pub fn survivor_criteria(mut self, survive: SurvivalCriteria) -> Self {
        self.survivor_criteria = survive;
        self
    }

    /// give the population a way to pick the parents, if none is supplied 
    /// then default to biased random genomes
    pub fn parental_criteria(mut self, parents: ParentalCriteria) -> Self {
        self.parental_criteria =parents;
        self
    }
}




/// This is a default config implementation which 
/// needs to be set for the population to evolve 
/// with speciation. These numbers need to be 
/// set for the evolution to work correctly
impl Config {
    pub fn new() -> Self {
        Config {
            inbreed_rate: 0.0,
            crossover_rate: 0.0,
            distance: 0.0,
            species_target: 0
        }
    }
}



impl Stagnant {
    pub fn new(target_stagnation: usize, cleaners: Vec<Genocide>) -> Self {
        Stagnant {
            target_stagnation,
            current_stagnation: 0,
            previous_top_score: 0.0,
            cleaners
        }
    }
}
