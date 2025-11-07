<!-- Author: Codex & Jörn -->

This document describes an experiment idea, or rather a dataset idea that can be reused by many other experiments.

We want to create a dataset with many different polytopes as rows, and various computed quantities as columns. We can use different sources for (4d, convex, star-shaped) polytopes, importantly different random distributions to sample from, and enumerations of certain classes, and lists of special polytopes from the literature or that are interesting for other reasons. Generator design, interface conventions, and the growing catalogue now live in [Random Polytope Generators](./random-polytopes.md#random-polytopes); this page focuses on how the atlas experiment consumes those generators.

## Config Format

- JSON configs (see `configs/atlas/test.json`) enumerate `sources`, each with a `name` and an explicit `rows` count. Enumerative sources may use `max_rows` when they halt naturally (the build stage treats `max_rows` as the emitted row count for reproducible datasets).
- Global knobs (`seed`, output path, provenance) remain at the top level; there is no single `rows_total` anymore. Scaling up/down is done by editing the per-source integers.
- The build stage records `source`, `source_row`, and `replay_token` columns so downstream analysis knows exactly how many rows originated from each generator.

Random distributions mainly distinguish themselves by whether we target a half-space or vertex count, whether there are constraints we respect, e.g. an in-sphere, or a circum-sphere, or a lagrangian product structure.
We don't care about affine symplectomorphisms, so volume normalization doesn't matter, and at most helps with numerics. We care about star-shapedness, so we recenter, or discard invalid polytopes during generation.

We may want to enumerate the class of lagrangian products of 2d regular polygons with a rational rotation angle. The viterbo counterexample is of this form, and we may find more counterexamples this way.
This is in addition to random lagrangian products.

Another random class of interest are Mahler-conjecture polygons, i.e. products $K x K^o$ for 2d convex bodies K. We can sample K randomly (see the Mahler sampler in [Random Polytope Generators](./random-polytopes.md#random-polytopes)). It's proven that the Viterbo Conjecture is equivalent to the Mahler Conjecture in this case, and the Mahler Conjecture is known to be true in 2d, so these polytopes should all have systolic ratio <= 1.

For special polytopes, we mainly care about
- the viterbo counterexample by Heim-Kisliv (2024)
- the recentered orthogonal unit simplex (0, e_1, e_2, e_3, e_4)
- the hypercube [-1,1]^4
- the cross polytope conv(±e_1, ±e_2, ±e_3, ±e_4)
- the symplectic disc, i.e. any *symplectic* instead of lagrangian product of two 2d polytopes, since 2d polytopes are symplectomorphic to discs (which only matters for *symplectic* products, not for lagrangian products, since in latter case there's no symplectomorphism that homotopies both factors at the same time to discs)

There's other classes related to c_ECH capacity, mayyybe we will add those later.

For columns, we want to compute
- the geometric representation: half-spaces, vertices
- volume
- EHZ capacity
  - via minkowski billiard algorithm for lagrangian products
  - via our combinatorical algorithm in the 2-face graph
  - via the HK linear programming algorithm
- the minimum action orbit
  - again via the above algorithms
- the systolic ratio = c_EHZ^2 / (2 vol)
- future: the EHZ capacity spectrum, i.e. the list of lowest-action orbits, not just the minimum one; we do not have an algorithm for this yet, though it's easy to get a basic one by dropping the rotation constraint, rethinking the exclusion of paths along 1-faces, rethinking the "visit each 3-face at most once" theorem, and then only using a constant action cutoff to enumerate all orbits instead of just the minimum one.

It may make sense to look at umap/t-sne projections of the dataset. Potential metrics of similarity between polytopes include
- Hausdorff distance between boundaries
- for equal vertex count: Sum of squared distances, minimized over a vertex matching
- Hausdorff distance, minimized over affine symplectomorphisms (might require best-guess symplectomorphisms via gradient descent)
It makes sense to normalize volume to 1 first when computing distances.

We can augment distance metrics by feature vectors if we e.g. want to further distinguish similar polytopes but different systolic ratios or equivalently capacities after normalizing volume.

The dataset is intended to be used for
- scanning the dataset for more counterexamples to the Viterbo Conjecture that are different from the known one
- fitting regression and classification models
- using distance metrics to simplify the regions in polytope-space and investigate whether there are large components of counterexamples etc.
- just having data to immerse oneself in
- benchmarking algorithms against each other on a large variety of polytopes
- it encourages high code quality, and working math algorithms with optimized performance, which is useful for other experiments as well

For E2E testing we simply create ~10 rows, maybe 1-3 per random distribution to balance size distribution, after picking 1 from any enumeration scheme plus the special polytopes. Same column, row format, just fewer rows.

We may also have a profiled version, where the full dataset creation is profiled to identify hotspots. If we include extra benchmark columns, we can then connect polytope size and distribution to the runtime cost of different algorithms.

The building of large datasets also matters for the simplex-dataset, that may yield a computer-assisted proof of the viterbo conjecture for simplices.
