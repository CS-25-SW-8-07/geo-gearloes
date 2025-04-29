use burn::{
    nn::{
        gru::{Gru, GruConfig},
        pool::{AdaptiveAvgPool2d, AdaptiveAvgPool2dConfig},
        Dropout, DropoutConfig, Linear, LinearConfig, Relu,
    },
    prelude::*,
};

#[derive(Debug, Clone)]
pub struct Model {
    pub output_dim: usize,
    pub gru1: Gru<burn::backend::Wgpu<f32, i32>>,
    pub gru2: Gru<burn::backend::Wgpu<f32, i32>>,
    pub pool: AdaptiveAvgPool2d,
    pub dropout: Dropout,
    pub linear1: Linear<burn::backend::Wgpu<f32, i32>>,
    pub linear2: Linear<burn::backend::Wgpu<f32, i32>>,
    pub activation: Relu,
}

#[derive(Config, Debug)]
pub struct ModelConfig {
    #[config(default = 128)]
    pub hidden_size: usize,
    #[config(default = "0.5")]
    pub dropout: f64,
}
