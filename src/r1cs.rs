use std::marker::PhantomData;
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Cell, Value, Layouter, SimpleFloorPlanner},
    plonk::{Advice, Assigned, Circuit, Column, ConstraintSystem, Error, Fixed, Instance},
    poly::Rotation,
};

// a*b-c = 0
#[derive(Debug, Clone)]
struct R1CSConfig {
    a: Column<Advice>,
    b: Column<Advice>,
    c: Column<Instance>,
}

#[derive(Debug, Clone)]
struct R1CSChip<F: FieldExt> {
    config: R1CSConfig,
    marker: PhantomData<F>,
}

impl<F: FieldExt> R1CSChip<F> {
    fn new(config: R1CSConfig) -> Self {
        R1CSChip {
            config,
            marker: PhantomData,
        }
    }
}

trait R1CSComposer<F: FieldExt> {
    fn assign_a(
        &self,
        layouter: &mut impl Layouter<F>,
        a: Vec<F>
    ) -> Result<(), Error>;

    fn assign_b(
        &self,
        layouter: &mut impl Layouter<F>,
        b: Vec<F>
    ) -> Result<(), Error>;
}

impl<F: FieldExt> R1CSComposer<F> for R1CSChip<F> {

    fn assign_a(
        &self,
        layouter: &mut impl Layouter<F>,
        a: Vec<F>
    ) -> Result<(), Error>
    {
        layouter.assign_region(
            || "sc",
            |mut region| {
                for i in 0..a.len() {
                    region.assign_advice(|| "a", self.config.a, i, || Value::known(a[i]))?;
                }
                Ok(())
            },
        )
    }

    fn assign_b(
        &self,
        layouter: &mut impl Layouter<F>,
        b: Vec<F>
    ) -> Result<(), Error>
    {
        layouter.assign_region(
            || "sc",
            |mut region| {
                for i in 0..b.len() {
                    // @todo check if offset should be 0 or i.
                    region.assign_advice(|| "b", self.config.b, i, || Value::known(b[i]))?;
                }
                Ok(())
            },
        )
    }
}

#[derive(Default)]
struct R1CSCircuit<F: FieldExt> {
    a: Vec<F>,
    b: Vec<F>,
    c: Vec<F>,
}

impl<F: FieldExt> Circuit<F> for R1CSCircuit<F> {
    type Config = R1CSConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let a = meta.advice_column();
        let b = meta.advice_column();

        // meta.enable_equality(l);

        // let is_hash = meta.fixed_column();
        // let hash = meta.instance_column();

        let c = meta.instance_column();
        // meta.enable_equality(c);

        meta.create_gate("c-a*b", |meta| {
            let a = meta.query_advice(a, Rotation::cur());
            let b = meta.query_advice(b, Rotation::cur());
            let c = meta.query_instance(c, Rotation::cur());

            vec![c - (a*b)]
        });

        R1CSConfig {
            a,
            b,
            c,
        }
    }

    fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<F>) -> Result<(), Error> {
        let cs = R1CSChip::new(config);

        // let a = self.a;
        // let b = self.b;

        cs.assign_a(&mut layouter, self.a.clone())?;
        cs.assign_b(&mut layouter, self.b.clone())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::R1CSCircuit;
    use halo2_proofs::circuit::Value;
    use halo2_proofs::halo2curves::bn256::Fr as Fp;
    use std::env;
    #[test]
    fn test_r1cs() {
        env::set_var("RUST_BACKTRACE", "full");
        use halo2_proofs::dev::MockProver;

        let k = 4;
        let a = vec![Fp::from(5), Fp::from(4), Fp::from(3)];
        let b = vec![Fp::from(3), Fp::from(4), Fp::from(10)];
        let c = vec![Fp::from(15), Fp::from(16), Fp::from(30)];

        let circuit = R1CSCircuit {
            a: a,
            b: b,
            c: c.clone(),
        };

        let public_inputs = vec![c];

        let prover = MockProver::run(k, &circuit, public_inputs).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }

    // #[cfg(feature = "dev-graph")]
    // #[test]
    // fn plonk_layout() {
    //     use plotters::prelude::*;

    //     let root = BitMapBackend::new("plonk-layout.png", (1024, 3096)).into_drawing_area();
    //     root.fill(&WHITE).unwrap();
    //     let root = root.titled("Plonk Layout", ("sans-serif", 60)).unwrap();

    //     let circuit = R1CSCircuit::<Fp> {
    //         x: Value::unknown(),
    //         y: Value::unknown(),
    //         constant: Fp::from(7),
    //         constant_fixed: Fp::from(10),
    //     };
    //     halo2_proofs::dev::CircuitLayout::default()
    //         .mark_equality_cells(true)
    //         .show_equality_constraints(true)
    //         .render(4, &circuit, &root)
    //         .unwrap();

    //     let dot_string = halo2_proofs::dev::circuit_dot_graph(&circuit);
    //     println!("---{}---", dot_string); // --> bug: is empty
    //     // let mut dot_graph = std::fs::File::create("circuit.dot").unwrap();
    //     // std::io::Write::write_all(&mut dot_graph, dot_string.as_bytes()).unwrap();
    // }
}
