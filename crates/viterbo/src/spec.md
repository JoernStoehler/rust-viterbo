# Specification

This is an additional layer between high level ideas and proof sketches, and the final code.
We basically sum up all our convenience definitions and lemmas, and provide hints for codex agents to translate them into code.
Since Rust does not offer dependent types such as lean does, we use simple rust types (usually not even generics is needed) and comments to document additional constraints/contracts. We use debugasserts to verify at runtime that the dependent types work out.
Definitions usually result in types and bool predicates that can be debugasserted.
Lemmas usually result in algorithms that convert between types, with input contracts and output contracts that enrich the types.
Some lemmas encode a dependent type cast that's a noop on Rust types, i.e. they simply state that some input has additional properties i.e. that certain debugasserts will work as well.

## Context
We for now focus on scenarios where's a single 4d polytope $K$ given, one that's finite, non-empty, convex, non-degenerate, star-shaped (contains origin), but not necessarily generic or without degenerate lower-dimensional skeleton (e.g. 5 facets touching in a vertex instead of the normal 4 facets touching a vertex).

## Basic Definitions

- body = 4d polytope = our $K$, assumes the various properties.  
- skeleton consists of 3d facets, 2d ridges, 1d edges, 0d vertices
- we disallow body, facets, ridges, edges, vertices with empty interior
- facet normals = 4d outward normal vectors on the facets
- ridge tangent space = spanned by the tangent vectors of a ridge, subspace of R^4
- basis of tangent space = ordered basis (u1,u2) of the tangent space
- oriented orthonormal basis = fulfills omega(u1,u2)>0 and u1,u2 are orthonormal
- chart of a ridge = an affine map R^4 -> R^2 such that (u1,u2) -> (e1,e2); note that omega_standard(e1,e2)>0 so this is orientation preserving
- chart domain = the interior of the 2d polytope in R^2 that is mapped by the chart onto the interior of the ridge in 4d
- Lemma: a ridge with a lagrangian tangent space has no oriented basis; other ridges have at least one
- Reeb vector on a facet R = J * n
- Lemma: Reeb vector is non-zero and tangent to the facet
- a ridge on a facet is either transversal or tangent to the Reeb vector; transversal ridges are either exits or entrances for the Reeb flow on the fact
- Lemma: every ridge has two adjacent facets; a lagrangian ridge is tangent to both Reeb flows; a non-lagrangian ridge has one facet where the flow goes into the ridge and one facet where the flow comes out of the ridge
  Proof sketch: a ray from the ridge in direction $R1=J n1$ points into facet 1 iff $0 > <J n1, n2> = - <n1, J n2> = - omega(n1,n2)$ i.e. iff $omega(n1, n2) > 0$. It's tangential iff 0, and points out of facet 1 if $< 0$. If $omega(n1,n2)=0$ then $n1,n2$ span a lagrangian subspace, and the orthogonal space is also lagrangian, i.e. the ridge is lagrangian.
- the partially defined flow map $\psi_{i j}$ maps any point on the ridge $i$ to where its Reeb flow hits the ridge $j$ when going through the (implied) single facet they are both adjacent to; it's domain and image are convex subsets of the ridges $i,j$ (intersection with pullback or with pushforward respectively); the domain and image may be empty even if $i$ is an entrance and $j$ an exit on the facet (example in 2d: parallelogram where upwards flow does not connect bottom line and top line)
- we can also view the flow map via the charts as a partial affine map $R^2 \to R^2$.
- lemma: for lagrangian ridges, all their flow maps are empty; for non-lagrangian ridges, at least one flow map with it as source and one with it as target must be non-empty; the domains of flow maps are a disjoint partition of the ridge (up to a zero measure set of the edge and vertex pre-images); similar for the images;
- lemma: the flow map is orientation preserving, i.e. the pushforward of the oriented basis of $i$ has positive symplectic form $\omega(u1^*, u2^*)>0$ i.e. the same orientation as the oriented basis of $j$. for the flow map expressed on teh charts, this means its 2x2 matrix has positive determinant.

## Orbits
- to us, a trajectory is a piecewise linear curve along Reeb vectors on facets and crossing through ridges, and optionally following edges in the one direction that's positive wrt all 3 adjacent Reeb vectors, and crossing through vertices
- orbit = closed trajectory aka closed characteristics
- action = the standard action integral A(gamma) = int_gamma alpha_st where alpha_st = 1/2 sum x_k dy_k - y_k dx_k iirc
- conjecture: in the generic case there is a minimum action orbit that does not cross vertices and does not follow edges and does not cross edges (uncomfirmed! asserts pls!)
- theorem (HK2017 & CH2021): there is a minimum action orbit that visits every facet at most once (important!)
- rotation = defined for the CZ index = ceil(rotation) + floor(rotation)
- theorem: CZ index = 3 for the minimum action orbit; <=> 1 < rotation < 2
- lemma: for our setting (star-shaped, convex, non-degenerate) the action is increasing along the trajectory
- lemma: similary the rotation is increasing along the trajectory (todo: find citation)
- the action/rotation increment can be defined for a segment that goes from ridge to ridge along one facet, it is non-negative
- lemma: the rotation increment is independent of the startpoint in the domain of $\psi_{i j}$, the action increment is an affine scalar-valued function in the start point or in the end point

## Ridge Graph
- we define a graph with (non-empty) ridges as nodes, and an oriented edge whenever $\psi_{i j}$ is defined and non-empty
- we can attach data:
  - the incoming/outgoing edges of a node all correspond to the same facet respectively, namely the facet that flows into/outof the ridge
  - for each edge we have the action & rotation increment
- Lemma: between two nodes there is at most one edge; no node is terminal or initial; there may or may not be a single connected component;
- Lemma: any generic orbit (not necessarily minimal; no 1-faces and 0-faces) has a cycle in the graph of what ridges it visits; there's at least one minimum action orbit where the cycle does not revisit a facet, and thus also does not revisit a ridge (though not revisitng facets may be stronger!).

## Algorithm
- we can now search via a mix of heuristics and depth-first search for a minimum action orbit
- outmost loop: iterate over start nodes, prune them from the tree after they are complete
- imagine (don#t build in memory) the tree of paths from the start node, with the zero-step path as root, and cycles as leaves
- we do a DFS for the best candidate
- for each cycle we can calculate the composed flow map from start to start, and look for a fixed point, and check admissability, and compute the action
- we can further prune the search tree massively:
- lemma: since action and rotation increments are non-negative, once a path exceeds the current best action or the bound $rotation<2$, we can ignore the subtree. note that we take the minimum action here over all admissable trajectories along the combinatorical path. the rotation is a scalar.
- lemma: if the path's start and end lie on the same facet, the only relevant part of the subtree is the immediate cyclic closure if it exists, the rest of the subtree can be ignored; this is bc at least one minimum action orbit exists that doesn#t revisit facets
- lemma: any subtree that has an end ridge that enters a facet that some other ridge in the path already entered can be ignored; same reason
- we speed up our search by formulating the DFS as recursive, with heuristical prioritization which child to explore next, and we cache intermediate values

Pseudo code:
- Given
  - path of nodes (= 2-faces = ridges) [1, 2, ..., k]
  - upper bound on the capacity
  - (recomputable if ommitted:) accumulated flow map across the edges between nodes, in the charts of the first to the last node
  - (recomputable if ommitted:) accumulated action functional, in the chart of the last node
  - (recomputable if ommitted:) accumulated rotation, scalar
  - (recomputable if ommitted:) the trajectory bundle that has the given combinatorics and stays below the upper bound on action, expressed as a convex subset of the last node's chart domain (trajectory endpoints)
  - (recomputable if omitted:) the set of "visited" facets, that are entered by the path's nodes (no overlap due to no-revisit-condition)
- Return
  - either: the orbit with lowest action among all orbits in this subtree that fulfill various conditions we know that at least one minimum action orbit also fulfills
  - or: None
- Algorithm:
  - use a heuristic for what order to recurse in
    - if we can close the path to a cycle: only do that
    - otherwise list all reachable nodes
    - omit nodes that enter an already visited facet
    - pick some ordering, e.g. lower bounds on action increments
  - after every recursive call that yields a new best candidate isntead of None, update the upper bound and remember the candidate
  - the arguments for the recursion are obtained with the auxiliary function below, skip recursion call if the arguments are None
  - return the candidate or None if none was ever found

- Given
  - ...
  - an oriented edge k -> k+1 to a new node k+1
- Return
  - ... or None
- Algorithm:
  - update the rotation using the edge's rotation increment
  - early exit if exceeds bound $rotation < 2$
  - add the action increment to the current action functional (same chart!)
  - compute the minimum of the action functional over the trajectory bundle endpoint set
  - early exit if it exceeds the upper bound on the action
  - intersect the trajectory endpoint set with the halfplane where the action functional stays below the bound
  - (redundant, never reached: early exit if no trajectories remain)
  - push forward the accumulated flow map along the flow k -> k+1
  - intersect with the k+1 chart domain
  - early exit if empty
  - push forward the accumulated action functional
  - extend the set of visited faces, extend the path of nodes
  - return

## Rounding Errors
- we are conservative wrt rounding errors: when we are about to exclude a search subtree, we only do so if it's exceeding admisssability cutoffs by epsilon; otherwise we continue down the subtree but flag it as "suspicious" i.e. we have to recalculate at the end using exact rational numbers whether it was okay to use. This is possible bc the combinatorics is discrete and we assume/round the polytope to rational coordinates. If it turns out that the orbit is not actually valid under rational numbers, we error, to indicate that a more careful algorithm must be used.
- conjecture: inadmissable numerical orbits of the kind we consider all are close to an exact admissable orbit, or close to an exact inadmissable orbit that goes outside the $\partial K$ boundary and has too high action. see HK2017 for some linear problem algorithm that doesn't track admissability (TODO: write up why HK implies what we conjecture here)
- corrolary: the algorithm still returns approximately up to rounding the true minimum action orbit and capacity
- 