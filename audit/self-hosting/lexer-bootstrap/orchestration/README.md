# Completed local S1 continuation

`receipt.json` is the unchanged final output of `run.py.gz`, invoked from the
isolated main Trident checkout with:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 ../measurements/lexer-v9-followup.py > ../measurements/lexer-v9-followup.log 2>&1
```

It waited for the actual first S1 build, then ran the complete C2→C3 build,
actual C2 semantic corpus and source-size checks. After the second build passed,
it checked the fixed point and launched the actual C3 semantic corpus.
Every child command completed with exit zero. The invoking process also exited
zero. Frozen compiler sources and the pinned runtime stayed unchanged; each
command kept its declared quotas and corpus expectations. Corpus receipts
separately identify their raw Rust reference builds.

`files.json` binds the exact coordinator source and raw logs; deterministic
gzip preserves their bytes. The final receipt records each actual command,
working directory, elapsed duration, exit status and log hash. The first-build
receipt SHA256 is `868ad429aa7d0b4867d10c5d59e04adc37c5179d19d537734e50155336fef0c6`.
The frozen S1 source is `77213171d39b88c5f41221912251cc4813ac2b11`;
individual receipts retain actual launch revisions and source hashes.

This establishes a complete local execution chain. The two clean origin
repetitions on each of six native platforms are the remaining SH6 acceptance
gate and have separate evidence.
