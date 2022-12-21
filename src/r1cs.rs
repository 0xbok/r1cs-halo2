use std::marker::PhantomData;
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Cell, Value, Layouter, SimpleFloorPlanner},
    plonk::{Advice, Assigned, Circuit, Column, ConstraintSystem, Error, Fixed, Instance, Selector},
    poly::Rotation,
};

// a*b-c = 0
#[derive(Debug, Clone)]
struct R1CSConfig {
    a: Column<Advice>,
    b: Column<Advice>,
    sel: Column<Fixed>,
    // c: Column<Instance>, // compiler warning: `c` is never read
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
    fn assign_advice(
        &self,
        layouter: &mut impl Layouter<F>,
        a: F,
        b: F,
    ) -> Result<(), Error>;
}

impl<F: FieldExt> R1CSComposer<F> for R1CSChip<F> {

    fn assign_advice(
        &self,
        layouter: &mut impl Layouter<F>,
        a: F,
        b: F,
    ) -> Result<(), Error>
    {
        layouter.assign_region(
            || "a and b",
            |mut region| {
                    region.assign_advice(|| "a", self.config.a, 0, || Value::known(a))?;
                    region.assign_advice(|| "b", self.config.b, 0, || Value::known(b))?;
                    region.assign_fixed(|| "sel", self.config.sel, 0, || Value::known(F::one()))?;
                Ok(())
            },
        )
    }
}

#[derive(Default)]
struct R1CSCircuit<F: FieldExt> {
    a: Vec<F>,
    b: Vec<F>,
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
        let c = meta.instance_column();
        let sel = meta.fixed_column();
        // meta.enable_equality(c);

        meta.create_gate("sel*(c-a*b)", |meta| {
            let a = meta.query_advice(a, Rotation::cur());
            let b = meta.query_advice(b, Rotation::cur());
            let c = meta.query_instance(c, Rotation::cur());
            let sel = meta.query_fixed(sel, Rotation::cur());

            vec![sel*(c - (a*b))]
        });

        R1CSConfig {
            a,
            b,
            sel,
            // c,
        }
    }

    fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<F>) -> Result<(), Error> {
        let cs = R1CSChip::new(config);

        for i in 0..self.a.len() {
            cs.assign_advice(&mut layouter, self.a[i], self.b[i])?;
        }

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
            // c: c.clone(),
        };

        let public_inputs = vec![c];

        let prover = MockProver::run(k, &circuit, public_inputs).unwrap();
        prover.assert_satisfied();
    }

    #[cfg(feature = "dev-graph")]
    #[test]
    fn r1cs_layout() {
        use plotters::prelude::*;

        let root = BitMapBackend::new("r1cs-layout.png", (1024, 3096)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root.titled("R1CS Layout", ("sans-serif", 60)).unwrap();

        let circuit = R1CSCircuit::<Fp> {
            a: vec![Fp::from(1), Fp::from(2)],//Value::unknown(), Value::unknown()],
            b: vec![Fp::from(1), Fp::from(2)],//Value::unknown(), Value::unknown()],
            // sel: vec![Fp::from(1), Fp::from(1)],
            // c: vec![Fp::from(1), Fp::from(2)],//Value::unknown(), Value::unknown()],
        };
        halo2_proofs::dev::CircuitLayout::default()
            .mark_equality_cells(true)
            .show_equality_constraints(true)
            .render(4, &circuit, &root)
            .unwrap();

        let dot_string = halo2_proofs::dev::circuit_dot_graph(&circuit);
        println!("---{}---", dot_string); // --> bug: is empty
        // let mut dot_graph = std::fs::File::create("circuit.dot").unwrap();
        // std::io::Write::write_all(&mut dot_graph, dot_string.as_bytes()).unwrap();
    }
}
