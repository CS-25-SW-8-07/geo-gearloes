use burn::{
    module::Param,
    nn::{gru::Gru, pool::AdaptiveAvgPool2d, Dropout, GateController, Linear},
    prelude::Backend,
    tensor::Tensor,
};
use sqlx::{pool::PoolConnection, Postgres};

use rusty_roads::burn::Model;

use crate::error::DbError;

pub async fn fetch_model(mut conn: PoolConnection<Postgres>) -> Result<Vec<u8>, DbError> {
    let query: String = "SELECT model FROM model LIMIT 1".into();

    let data: (Vec<u8>,) = sqlx::query_as(&query).fetch_one(&mut *conn).await?;

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

fn combine_models<B: Backend>(models: &[Model<B>]) -> Model<B> {
    let gru1: Vec<Gru<B>> = models.into_iter().map(|x| x.gru1.clone()).collect();
    let gru2: Vec<Gru<B>> = models.into_iter().map(|x| x.gru2.clone()).collect();
    let pool: Vec<AdaptiveAvgPool2d> = models.into_iter().map(|x| x.pool.clone()).collect();
    let dropout: Vec<Dropout> = models.into_iter().map(|x| x.dropout.clone()).collect();
    let linear1: Vec<Linear<B>> = models.into_iter().map(|x| x.linear1.clone()).collect();
    let linear2: Vec<Linear<B>> = models.into_iter().map(|x| x.linear2.clone()).collect();

    Model {
        output_dim: models[0].output_dim,
        gru1: combine_grus(&gru1),
        gru2: combine_grus(&gru2),
        pool: combine_avg_pools(&pool),
        dropout: combine_dropouts(&dropout),
        linear1: combine_linear(&linear1),
        linear2: combine_linear(&linear2),
        activation: burn::nn::Relu::new(),
    }
}

fn combine_grus<B: Backend>(grus: &[Gru<B>]) -> Gru<B> {
    let update_gate: Vec<GateController<B>> =
        grus.into_iter().map(|x| x.update_gate.clone()).collect();
    let reset_gate: Vec<GateController<B>> =
        grus.into_iter().map(|x| x.reset_gate.clone()).collect();
    let new_gate: Vec<GateController<B>> = grus.into_iter().map(|x| x.new_gate.clone()).collect();

    Gru {
        update_gate: combine_gates(&update_gate),
        reset_gate: combine_gates(&reset_gate),
        new_gate: combine_gates(&new_gate),
        d_hidden: grus[0].d_hidden,
    }
}

fn combine_dropouts(dropouts: &[Dropout]) -> Dropout {
    Dropout {
        prob: (dropouts.into_iter().map(|x| x.prob).sum::<f64>() / dropouts.len() as f64),
    }
}

fn combine_avg_pools(avg_pools: &[AdaptiveAvgPool2d]) -> AdaptiveAvgPool2d {
    let init = AdaptiveAvgPool2d {
        output_size: [0, 0],
    };

    let mut avg_pool = avg_pools.into_iter().fold(init, |mut acc, x| {
        acc.output_size = acc
            .output_size
            .iter()
            .zip(x.output_size.iter())
            .map(|(a, b)| *a + *b)
            .collect::<Vec<usize>>()
            .try_into()
            .expect("huh");

        acc
    });

    let count = avg_pools.len();

    let _ = avg_pool
        .output_size
        .iter_mut()
        .map(|x| *x = *x / count)
        .collect::<Vec<_>>();

    avg_pool
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
    let empty_tensor: Tensor<B, D> = Tensor::zeros(tensors[0].shape(), &tensors[0].device());

    let new_tensor = tensors
        .into_iter()
        .fold(empty_tensor, |acc, x| acc.add(x.clone()));

    let scalar = tensors.len() as f64;

    new_tensor.div_scalar(scalar)
}

#[cfg(test)]
mod tests {
    use super::*;
    use burn::backend::Wgpu;
    use rand_chacha::ChaCha12Rng;
    use rand_core::{RngCore, SeedableRng};
    use std::rc::Rc;

    fn create_gru<B: Backend>(seed: u64) -> Gru<B> {
        let mut rng = ChaCha12Rng::seed_from_u64(seed);

        Gru {
            update_gate: create_gate(rng.next_u64()),
            reset_gate: create_gate(rng.next_u64()),
            new_gate: create_gate(rng.next_u64()),
            d_hidden: 2,
        }
    }

    fn create_gate<B: Backend>(seed: u64) -> GateController<B> {
        let mut rng = ChaCha12Rng::seed_from_u64(seed);

        GateController {
            input_transform: create_linear(rng.next_u64()),
            hidden_transform: create_linear(rng.next_u64()),
        }
    }

    fn create_param<B: Backend, const D: usize>(seed: u64) -> Param<Tensor<B, D>> {
        let mut rng = ChaCha12Rng::seed_from_u64(seed);

        let param = burn::module::Param::initialized(
            burn::module::ParamId::new(),
            create_tensor::<B, D>(rng.next_u64()),
        );

        param
    }

    fn create_tensor<B: Backend, const D: usize>(seed: u64) -> Tensor<B, D> {
        // let mut binding = ChaCha12Rng::seed_from_u64(seed);

        let mut rng = Rc::new(ChaCha12Rng::seed_from_u64(seed));

        let device = Default::default();

        match D {
            2 => {
                let mut data: [[f32; 5]; 5] = [[0.; 5]; 5];

                data.iter_mut().for_each(|x| {
                    x.iter_mut().for_each(|y| {
                        *y = random_range(Rc::get_mut(&mut rng).unwrap(), -10.0..10.0);
                    })
                });

                return Tensor::<B, D>::from_data(data, &device);
            }
            1 => {
                let mut data: [f32; 5] = [0.; 5];

                data.iter_mut().for_each(|y| {
                    *y = random_range(Rc::get_mut(&mut rng).unwrap(), -10.0..10.0);
                });
                return Tensor::<B, D>::from_data(data, &device);
            }
            _ => {
                return Tensor::ones([5, 5], &device);
            }
        }
    }

    fn create_pool(seed: u64) -> AdaptiveAvgPool2d {
        let mut rng = ChaCha12Rng::seed_from_u64(seed);

        AdaptiveAvgPool2d {
            output_size: [
                (rng.next_u64() % 10) as usize,
                (rng.next_u64() % 10) as usize,
            ],
        }
    }

    fn create_dropout(seed: u64) -> Dropout {
        let mut rng = ChaCha12Rng::seed_from_u64(seed);

        Dropout {
            prob: f64::from_bits(rng.next_u64()),
        }
    }

    fn create_linear<B: Backend>(seed: u64) -> Linear<B> {
        let mut rng = ChaCha12Rng::seed_from_u64(seed);

        Linear {
            weight: create_param(rng.next_u64()),
            bias: Some(create_param(rng.next_u64())),
        }
    }

    fn create_model<B: Backend>(seed: u64) -> Model<B> {
        let mut rng = ChaCha12Rng::seed_from_u64(seed);

        Model {
            output_dim: 2,
            gru1: create_gru(rng.next_u64()),
            gru2: create_gru(rng.next_u64()),
            pool: create_pool(rng.next_u64()),
            dropout: create_dropout(rng.next_u64()),
            linear1: create_linear(rng.next_u64()),
            linear2: create_linear(rng.next_u64()),
            activation: burn::nn::Relu::new(),
        }
    }

    use std::ops::{Add, Div, Sub};

    fn average_two<
        T: IntoIterator<Item = U>,
        U: Add<Output = T::Item> + Div<Output = U> + From<u16>,
    >(
        fst: T,
        snd: T,
    ) -> impl Iterator<Item = U> {
        fst.into_iter()
            .zip(snd.into_iter())
            .map(|(a, b)| (a + b) / 2_u16.into())
    }

    fn check_result<T: IntoIterator<Item = U> + std::fmt::Debug, U: PartialEq>(
        data: T,
        result: T,
    ) -> bool {
        dbg!(&data);
        dbg!(&result);
        result
            .into_iter()
            .zip(data.into_iter())
            .fold(true, |acc, (a, b)| acc && a.eq(&b))
    }

    fn random_range<T>(rng: &mut ChaCha12Rng, range: std::ops::Range<T>) -> T
    where
        T: Sub<Output = T> + Into<f32> + From<f32> + std::marker::Copy,
    {
        let number = f32::from_bits(rng.next_u32());

        ((number % (range.end - range.start).into()) + range.start.into()).into()
    }

    #[test]
    fn merge_models() {
        let model1 = create_model::<Wgpu>(4);
        let model2 = create_model::<Wgpu>(3);

        let data = vec![model1.clone(), model2.clone()];

        let result = combine_models(&data);

        dbg!(&model1
            .linear1
            .bias
            .clone()
            .unwrap()
            .into_value()
            .into_data()
            .into_vec::<f32>()
            .unwrap());
        dbg!(&model2
            .linear1
            .bias
            .clone()
            .unwrap()
            .into_value()
            .into_data()
            .into_vec::<f32>()
            .unwrap());
        dbg!(&result
            .linear1
            .bias
            .clone()
            .unwrap()
            .into_value()
            .into_data()
            .into_vec::<f32>()
            .unwrap());

        let check_linear_2d: Vec<f32> = average_two(
            model1
                .linear1
                .weight
                .into_value()
                .into_data()
                .into_vec::<f32>()
                .unwrap(),
            model2
                .linear1
                .weight
                .into_value()
                .into_data()
                .into_vec::<f32>()
                .unwrap(),
        )
        .collect();

        let check_linear_1d: Vec<f32> = average_two(
            model1
                .linear1
                .bias
                .unwrap()
                .into_value()
                .into_data()
                .into_vec::<f32>()
                .unwrap(),
            model2
                .linear1
                .bias
                .unwrap()
                .into_value()
                .into_data()
                .into_vec::<f32>()
                .unwrap(),
        )
        .collect();

        let check_avg_pool: Vec<usize> =
            average_two(model1.pool.output_size, model2.pool.output_size).collect();

        assert!(check_result(check_avg_pool, result.pool.output_size.into()));

        assert_eq!(
            (model1.dropout.prob + model2.dropout.prob) / 2.,
            result.dropout.prob
        );

        assert!(check_result(
            check_linear_1d,
            result
                .linear1
                .bias
                .unwrap()
                .into_value()
                .into_data()
                .into_vec::<f32>()
                .unwrap()
        ));

        assert!(check_result(
            check_linear_2d,
            result
                .linear1
                .weight
                .into_value()
                .into_data()
                .into_vec::<f32>()
                .unwrap()
        ));
    }
}
