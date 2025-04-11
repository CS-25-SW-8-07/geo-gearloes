use burn::{
    module::Param,
    nn::{GateController, Linear},
    prelude::Backend,
    tensor::Tensor,
};
use sqlx::{pool::PoolConnection, Postgres};

use crate::error::DbError;

pub async fn fetch_model(mut conn: PoolConnection<Postgres>) -> Result<Vec<u8>, DbError> {
    let query: String = "SELECT model FROM model LIMIT 1".into();

    let data: (Vec<u8>,) = sqlx::query_as(&query).fetch_one(&mut *conn).await?;

    // Deserialize here

    Ok(data.0)
}

pub async fn update_model(
    mut conn: PoolConnection<Postgres>,
    model: Vec<u8>,
) -> Result<(), DbError> {
    todo!()
}

fn combine_gates<B: Backend>(gates: &[GateController<B>]) -> GateController<B> {
    let input_transforms: Vec<Linear<B>> = gates
        .into_iter()
        .map(|x| x.input_transform.clone())
        .collect();
    let hidden_transforms: Vec<Linear<B>> = gates
        .into_iter()
        .map(|x| x.hidden_transform.clone())
        .collect();

    GateController {
        input_transform: combine_linear(&input_transforms),
        hidden_transform: combine_linear(&hidden_transforms),
    }
}

fn combine_linear<B: Backend>(linears: &[Linear<B>]) -> Linear<B> {
    let weights: Vec<Param<Tensor<B, 2>>> = linears.into_iter().map(|x| x.weight.clone()).collect();
    let bias: Option<Vec<Param<Tensor<B, 1>>>> =
        linears.into_iter().map(|x| x.bias.clone()).collect();

    Linear {
        weight: combine_params(&weights),
        bias: bias.map_or(None, |x| Some(combine_params(&x))),
    }
}

fn combine_params<B: Backend, const D: usize>(
    params: &[Param<Tensor<B, D>>],
) -> Param<Tensor<B, D>> {
    let tensors: Vec<Tensor<B, D>> = params.into_iter().map(|x| x.val()).collect();

    Param::initialized(params[0].id, combine_tensors(&tensors))
}

fn combine_tensors<B: Backend, const D: usize>(tensors: &[Tensor<B, D>]) -> Tensor<B, D> {
    let empty_tensor: Tensor<B, D> = Tensor::empty(tensors[0].shape(), &tensors[0].device());

    let new_tensor = tensors
        .into_iter()
        .fold(empty_tensor, |acc, x| acc.add(x.clone()));

    let scalar = tensors.len() as f64;

    new_tensor.div_scalar(scalar)
}

