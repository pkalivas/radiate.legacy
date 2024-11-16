

extern crate radiate;
extern crate serde_json;

use std::error::Error;
use std::time::Instant;
use radiate::prelude::*;


fn main() -> Result<(), Box<dyn Error>> {
       
    let thread_time = Instant::now();
    let mut net = Neat::new()
        .input_size(2)
        .dense(7, Activation::Relu)
        .dense(7, Activation::Relu)
        .dense(1, Activation::Sigmoid);
        
    let xor = XOR::new();
    let max_iter = 200;
    net.train(&xor.inputs, &xor.answers, 0.1, Loss::Diff, |iter, loss| {
        println!("epoch: {:?} loss: {:?}", iter, loss);
        iter == max_iter
    })?;
    
    xor.show(&mut net);

    let final_time = thread_time.elapsed().as_millis();
    println!("Time in millis: {}", final_time);

    Ok(())
}

#[derive(Debug)]
pub struct XOR {
    inputs: Vec<Vec<f32>>,
    answers: Vec<Vec<f32>>
}

impl XOR {
    pub fn new() -> Self {
        XOR {
            inputs: vec![
                vec![0.0, 0.0],
                vec![1.0, 1.0],
                vec![1.0, 0.0],
                vec![0.0, 1.0],
            ],
            answers: vec![
                vec![0.0],
                vec![0.0],
                vec![1.0],
                vec![1.0],
            ]
        }
    }


    fn show(&self, model: &mut Neat) {
        println!("\n");
        for (i, o) in self.inputs.iter().zip(self.answers.iter()) {
            let guess = model.forward(&i).unwrap();
            println!("Guess: {:.2?} Answer: {:.2}", guess, o[0]);
        }
    }

}



