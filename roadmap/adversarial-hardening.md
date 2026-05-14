---
status: draft
author: mastercyb
area: AI
planned: 128K
---

# Adversarial Compiler Hardening

## Motivation

A neural compiler is only as reliable as the distribution of programs it was trained on. If the training corpus contains mostly simple, well-structured programs, the compiler may perform poorly on adversarial inputs — programs specifically designed to expose blind spots. Without systematic adversarial testing, blind spots accumulate silently, discovered only when they cause problems in production.

Adversarial hardening embeds the test-and-fix loop directly into the compiler's development process. A generator creates programs designed to defeat the compiler. The compiler trains on these programs. The generator adapts. The equilibrium reached is a compiler with no systematic weaknesses — every weak point was found and fixed during development. Simultaneously, an equivalence checker stress tester generates "almost equivalent" TASM pairs to verify that the correctness oracle catches discrepancies. Zero false positives is the target.

## Design

### Adversarial Program Generation (GAN-like Loop)

The adversary is a neural generator trained to find programs where the neural compiler fails to optimize:

```
Each epoch:
  1. Generator creates 100 Trident programs (varied structure, operation mix)
  2. Neural compiler optimizes each program
  3. Reference optimizer (deterministic algebraic passes only) also optimizes each
  4. Compare: improvement_ratio = neural_cost / reference_cost
  5. Adversary reward = programs where improvement_ratio > 1.2 (compiler gets worse)
  6. Compiler trains on the adversarial programs with highest reward
  7. Generator updates to find NEW failures (different from current failures)
```

The key: the generator is NOT trying to break correctness. The generated programs are valid Trident programs. The adversary's goal is to find programs where the neural compiler underperforms the reference — where it fails to find optimizations that a simpler, more conservative compiler would find.

This is adversarial in the machine learning sense (like a GAN), not in the security sense. The adversary's product is a training curriculum, not an attack.

### Convergence and the Quality Gate

The GAN loop converges when the adversary cannot find programs where the compiler underperforms by more than the target threshold (e.g., improvement_ratio > 1.05, i.e., within 5% of optimal). At convergence:

- Every common failure mode has been found and fixed
- The compiler handles the adversary's best attempts with <5% overhead
- The adversary's training loss has plateaued

This convergence state is the quality gate. A compiler that has reached adversarial equilibrium is demonstrably robust — not just tested, but hardened against systematic weaknesses.

### Feeding the Identity Explorer

The adversarial programs that most defeat the compiler are programs containing instruction patterns where no known algebraic identity applies. These programs reveal gaps in the rule database. Each such program becomes a priority target for the algebraic identity explorer:

```
Adversarial program → no known identity matches → 
    → identity explorer focuses GFlowNet on this pattern →
    → new identity discovered (or confirmed nonexistent) →
    → rule database updated
```

The adversarial loop and the identity explorer form a discovery pipeline: the adversary finds unsolved patterns, the explorer solves them.

### Equivalence Checker Stress Testing

Alongside the adversarial program generator, an equivalence checker stress tester validates the correctness oracle. The tester generates "near-equivalent" TASM pairs — programs that agree on 99.99% of inputs but differ on specific edge cases:

**Generation method**: Start with a correct TASM program. Apply a single mutation:
- Change one constant operand to a slightly different value
- Swap two instructions that should not be swappable (subtle dependency violation)
- Remove a modular reduction that is usually redundant but fails for specific inputs

Each mutation is designed to be hard to detect — the programs look equivalent on casual inspection and agree on random inputs with high probability.

**Testing protocol**:
```
For each (original, mutant) pair:
  1. Run equivalence checker: declares equivalent/inequivalent?
  2. If declared equivalent: FALSE POSITIVE (serious bug — track)
  3. If declared inequivalent: CORRECT (find the distinguishing input)
  4. Track false positive rate: must be zero
```

Target: zero false positives (mutant declared equivalent to original). Track false negatives (equivalent pairs declared inequivalent) separately — these are optimization opportunities (the checker is being overly conservative).

### Adversary Architecture

The adversary generator is a neural program generator:

```
Input: current rule database state (what patterns are covered)
      + compiler failure history (what programs caused failures)
Output: Trident program (as a TIR graph)

Architecture:
  Context encoder: encode rule database as a 64-dim vector
  History encoder: encode failure patterns as a 64-dim vector
  Program decoder: autoregressive TIR graph construction
    (same architecture as the program synthesis model)
```

The adversary learns: "given what the compiler currently knows, generate programs that exploit its gaps."

### Adversarial Programs as Training Data

Every adversarial program that causes compiler failure becomes a training example:
- For the neural compiler: train on these programs to fix the failure
- For the identity explorer: prioritize finding identities that cover these patterns
- For the equivalence checker: add to the stress test suite

This creates a virtuous cycle: adversarial failures become the training curriculum. The compiler grows stronger specifically where it is weakest.

## Key Tradeoffs

**Adversary effectiveness**: The adversary can only find programs that expose *current* weaknesses. As the compiler improves, the adversary must search harder. If the adversary saturates its own search capacity before finding new failures, the GAN loop stalls — the adversary has not truly exhausted the failure space, it just cannot find failures with its current architecture. A stronger adversary (more parameters, longer search) may find failures the current adversary misses.

**Correctness vs. optimization failures**: This proposal focuses on optimization failures (compiler is correct but slow) and equivalence checker precision (checker has zero false positives). Security-critical correctness failures (compiler generates wrong TASM) require a different testing strategy — one based on formal verification, not adversarial optimization.

**Adversary vs. random testing**: Random test programs also expose compiler failures. The adversary is specifically designed to find failures more efficiently than random testing — it learns the structure of failures and generates programs more likely to expose gaps. For early-stage hardening (first 1000 adversarial epochs), random testing may be competitive. The adversary becomes more valuable as the compiler matures.

**Computational budget**: The GAN loop requires substantial computation — each epoch involves 100 programs × full compilation + optimization + proving. For high-quality adversarial training, thousands of epochs are needed. This is ongoing background computation, not a one-time cost.

## Implementation Sketch

```rust
// hardening/adversarial.rs
pub struct AdversarialLoop {
    generator: ProgramGenerator,
    compiler: NeuralCompiler,
    reference: ReferenceCompiler,
    identity_explorer: &'static IdentityExplorer,
}

impl AdversarialLoop {
    pub fn run_epoch(&mut self) -> AdversarialReport {
        let programs = self.generator.generate(100);
        let mut failures = Vec::new();

        for prog in &programs {
            let neural_cost = self.compiler.compile_and_estimate(prog);
            let ref_cost = self.reference.compile_and_estimate(prog);
            let ratio = neural_cost / ref_cost;

            if ratio > 1.2 {
                failures.push((prog.clone(), ratio));
                self.identity_explorer.add_priority_target(prog);
            }
        }

        self.compiler.train_on(&failures);
        let reward = failures.iter().map(|(_, r)| r).sum::<f64>();
        self.generator.update(reward, &failures);

        AdversarialReport { failures, epoch_reward: reward }
    }
}
```

The adversarial loop runs continuously in the background. Its output — a stream of challenging programs — feeds both the compiler's training data and the identity explorer's priority queue. The equilibrium metric (improvement_ratio within 5% of 1.0 on 95% of adversarial programs) is the convergence criterion and the quality gate for compiler releases.
