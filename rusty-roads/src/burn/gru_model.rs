use burn::{
    nn::{
        gru::{Gru, GruConfig},
        pool::{AdaptiveAvgPool2d, AdaptiveAvgPool2dConfig},
        Dropout, DropoutConfig, Linear, LinearConfig, Relu,
    },
    prelude::*,
};

pub struct Model<B: Backend> {
    output_dim: usize,
    gru1: Gru<B>,
    gru2: Gru<B>,
    pool: AdaptiveAvgPool2d,
    dropout: Dropout,
    linear1: Linear<B>,
    linear2: Linear<B>,
    activation: Relu,
}

#[derive(Config, Debug)]
pub struct ModelConfig {
    #[config(default = 128)]
    pub hidden_size: usize,
    #[config(default = "0.5")]
    pub dropout: f64,
}