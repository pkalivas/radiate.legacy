pub mod population;
pub mod generation;
pub mod niche;
pub mod genocide;
pub mod survival;



/// Genome is what will actually be evolved through the engine, 
/// this is going to be whatever data structure should be optimized
pub mod genome {
    
    use super::environment::Envionment;
    use std::marker::Sized;
    use std::sync::{Arc, RwLock};

    pub trait Genome<T, E>
        where
            T: ?Sized + Send + Sync,
            E: ?Sized + Send + Sync
    {
        /// Crossover is the process of taking two types (T) and returning 
        /// a new type, this is done through some defined form of 
        /// mutation using the config type, or through crossover 
        /// where parts of one type are given to parts of the other and that resulting
        /// type is returned
        fn crossover(one: &T, two: &T, env: Arc<RwLock<E>>, crossover_rate: f32) -> Option<T> 
            where 
                T: Sized,
                E: Envionment + Sized;
        
        /// This is a measure of an evolutionary type's structure or topology - depending on what is being evolved.
        /// This is needed to split the members in their respective species - essentially it is 
        /// a measure of how far away two types are from each other in a genetic 
        /// sense. Think of something like how similar humans are to dolphins, this is a way to quantify that.
        fn distance(one: &T, two: &T, env: Arc<RwLock<E>>) -> f32;
        
        /// Genome needs to have a base implementation in order for one of the population options to be satisfied
        /// 
        /// This can probably be implemented in a generic way for default if the user doesn't want to
        /// implement it for their problem. 
        fn base(_: &mut E) -> T
            where T: Sized
        {
            panic!("Base not implemented.");
        }
    }
}



/// Environment represents overall settings for a genome, this can be statistics to be 
/// tracked through evolution, or things like mutation rates or global counters. This is 
/// injected into functions throughout the generational process so it is accessible globally as a
/// center point for the evolution. Note - if this is to be used a mutable in crossover or mutation, 
/// this will slow down the optimization process as it will have to be locked during the writing thus
/// having the variables in the implementation of this trait be readonly is preferred but isn't that big of a deal
pub mod environment {
    pub trait Envionment {
        
        /// Reset can be used to reset the environment after a certain event occurs,
        /// if not this is an empty default implementation
        fn reset(&mut self) { }
    
    }
}



/// Problem is the actual problem to be solved.
/// This is wrapped in an Arc pointer due to the problem not wanting to be 
/// copied through threads. This was done intentionally because I wanted to be able to
/// represent supervised, unsupervised, and general reinforcement learning problems. This
/// means if you are using a supervised system and have a large dataset to analyze, if this 
/// dataset is stored in the problem (as they should be), without an Arc pointer this large dataset would 
/// be copied multiple times and take up massive amounts of memory. The Arc allows us to keep only one version
/// of the problem and share that between threads. Note - this means everything in the problem and all it's data
/// is explicitly readonly 
pub mod problem {

    pub trait Problem<T> {

        /// empty can be a new for Self, or some sort of default value,
        /// just needed to create a population with base parameters 
        fn empty() -> Self;
        
        /// Solve is what actually solves the problem , given a solver (the genome type)
        /// use the data in the type implementing the problem to solve the problem and return
        /// the member's score. The result of this function is the member's fitness score 
        fn solve(&self, member: &mut T) -> f32;
    }
}