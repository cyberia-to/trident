---
status: draft
author: mastercyb
area: AI
planned: 128K
---

# Field-Native Evolutionary Neural Network Training

## Motivation

Neural networks require training. Training requires updating weights to minimize loss. Gradient descent is the standard method: compute gradients, update weights in the gradient direction. In floating-point arithmetic, this works cleanly. In Goldilocks field arithmetic, it breaks: field elements have no notion of "direction," gradients don't have meaningful magnitude in modular arithmetic, and the field's multiplicative structure is incompatible with the smooth optimization landscape gradient descent requires.

Evolutionary optimization sidesteps all of this. Evolution operates on populations of weight vectors. It selects survivors by fitness (low loss on training data), generates children by crossover and mutation, and repeats. No gradients required. The operations — comparison, conditional copy, random field element substitution — are pure field arithmetic. Every training step is a valid Triton VM trace. Training is provable.

## Design

### Population and Representation

```
POPULATION: N = 16 weight vectors
INDIVIDUAL: [Field; WEIGHT_COUNT]  — flat array of field elements
WEIGHT_COUNT: 91,008 for a 3-layer MLP with 64 hidden units (64×64 + 64 + 64×10 + 10)
MEMORY: 16 × 91,008 × 8 bytes = 11.6 MB total — fits in L2 cache on M4 Pro
```

Each individual is a weight vector. Fitness is measured on the training dataset: lower loss → higher fitness (higher fitness rank).

### Evolution Algorithm

```trident
// One generation:
for individual in population:
    for example in training_batch:
        output = inference(individual.weights, example.input)  // nn.trd
        individual.loss += mse(output, example.label)

sort population by loss (ascending)
survivors = population[0..N/4]  // top 25% survive

for i in 0..N:
    parent_a = random_choice(survivors)
    parent_b = random_choice(survivors)
    child = uniform_crossover(parent_a, parent_b)  // each weight from either parent
    child = mutate(child, rate: 0.01)               // replace 1% of weights randomly
    new_population[i] = child
```

### Performance

Each generation involves:
- $N \times \text{batch\_size} \times \text{inference\_ops}$ field operations for fitness evaluation
- $N \times \text{WEIGHT\_COUNT}$ field operations for crossover and mutation

For $N=16$, batch size 100, ~2000 inference ops per example:
- Fitness: $16 \times 100 \times 2000 = 3.2\text{M}$ field ops
- Evolution: $16 \times 91{,}008 \approx 1.5\text{M}$ field ops
- **Total: ~4.7M field ops per generation**

On M4 Pro with Metal GPU (field ops vectorized):
- ~50μs per generation
- 1,000 generations in ~50ms
- 10,000 generations in ~500ms

This speed makes evolutionary training interactive. The developer can run thousands of generations in under a second and observe convergence in real time.

### Mutation Strategy

Uniform crossover: each weight independently selected from either parent with probability 0.5. Preserves approximately half of each parent's structure.

Mutation: with probability `rate = 0.01` per weight, replace with a random field element from a predetermined distribution. The distribution is not uniform over the full field — it concentrates around small field values (corresponding to small signed integers in the signed convention) to maintain the approximation accuracy of the initial weights.

```trident
fn mutate(weights: [Field; N], rate: Field) -> [Field; N] {
    weights.map(|w| {
        if random() < rate {
            // Sample from small-value distribution:
            (random() % SMALL_RANGE) as Field  // values near 0 in signed convention
        } else {
            w
        }
    })
}
```

### Hybrid Path: Cold Start + Evolutionary Refinement

For faster convergence, an optional hybrid approach uses finite-difference gradients for the first phase:

**Phase 1 — Finite-difference cold start** (~1M gradient steps, ~5s):
```trident
fn finite_diff_gradient(weights: [Field; N], example: Example, h: Field) -> [Field; N] {
    weights.enumerate().map(|(i, w)| {
        let loss_plus  = loss(weights.with(i, w + h), example);
        let loss_minus = loss(weights.with(i, w - h), example);
        (loss_plus - loss_minus) * invert(h + h)  // approximation in field arithmetic
    })
}
```

Finite-difference in field arithmetic is noisy (the division by `2h` is exact in the field, but the loss difference may not be a good gradient estimate when `h` is not small in the signed sense). This phase provides rough initialization.

**Phase 2 — Evolutionary refinement** (~1000 generations, ~50ms):
Evolution from the cold-start population. The initialized weights provide a good starting point; evolution handles the remaining optimization that gradients cannot.

### Provable Training Steps

Every generation of the evolutionary algorithm is a Trident program execution. Every training step is a valid Triton VM trace. The proof of training proves:

- The population evolved from the previous generation according to the declared crossover and mutation rules
- Fitness was evaluated correctly on the declared training examples
- The best-performing individual at generation $G$ has the declared loss

This enables verifiable machine learning: a party who receives the final weights can verify, via STARK proof, that the weights were produced by honest training on the declared dataset.

## Key Tradeoffs

**Convergence rate**: Evolutionary optimization converges slower than gradient descent for smooth landscapes. For a 64-unit MLP, expect convergence in 1,000–10,000 generations (50ms–500ms). For larger networks (256 units), convergence may require 100,000 generations (5s). The hybrid path mitigates this significantly.

**Population diversity**: A population of 16 is small. With only 4 survivors per generation, genetic drift is significant — the population may converge prematurely to a local optimum. Increasing N to 64 (47MB total) mitigates drift at 4× memory cost. The exploration bonus in the mutation rate helps maintain diversity.

**Loss approximation**: The MSE and cross-entropy loss functions in `nn.trd` are field approximations of the floating-point originals. Approximation error in the loss function may cause the evolution to optimize for the wrong objective. The approximation must be validated against the floating-point version before training in production.

**Selection pressure**: With top-25% selection, the bottom 75% are discarded each generation. Selection pressure this high causes fast initial convergence but may prevent fine-tuning. An adaptive selection pressure (high early, low late) would improve convergence but complicates the implementation.

## Implementation Sketch

```trident
// evolutionary_training.trd (in nn/)

fn train_epoch<const N_WEIGHTS: Field>(
    population: [WeightVector<N_WEIGHTS>; 16],
    training_data: [(Vector<IN>, Vector<OUT>)],
) -> [WeightVector<N_WEIGHTS>; 16] {

    // Fitness evaluation
    let fitnesses = population.map(|individual| {
        training_data.iter().map(|(x, y)| {
            let pred = inference::<IN, OUT>(individual, x);
            mse(pred, y)
        }).sum()
    });

    // Selection: top 25%
    let ranked = sort_by_fitness(population, fitnesses);
    let survivors = ranked[0..4];

    // Reproduction
    (0..16).map(|_| {
        let parent_a = random_choice(survivors);
        let parent_b = random_choice(survivors);
        let child = uniform_crossover(parent_a, parent_b);
        mutate(child, 0.01)
    }).collect()
}
```

The training loop is a simple `for generation in 0..N_GENERATIONS { population = train_epoch(population, data); }`. Each epoch is a TASM trace. The final population contains the trained weights.
