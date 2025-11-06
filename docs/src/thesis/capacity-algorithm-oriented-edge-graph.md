<!-- Author: Codex & Jörn -->

TODO: replace this file with actual content.

We want to implement the following algorithm to compute the EHZ capacity for a convex star-shaped non-degenerate polytope $K \subset \mathbb{R}^{4}$.

Input:
- The extreme points of $K$.
- The half-spaces defining $K$.

Output:
- The EHZ capacity of $K$.
- A minimum action closed characteristic on $\partial K$, which is a piecewise linear curve with specified vertices on the 2-faces of $K$ and implied segments on the 3-faces of $K$.

Algorithm:
**Phase 1: Pre-processing**
1. Build the 3-face graph of $K$.
   - Nodes correspond to 3-faces of $K$.
   - Edges correspond to two 3-faces sharing a common 2-face.
   - Orientation of edges is determined by which direction the two Reeb flows of the two 3-faces point towards the common 2-face. Due to convexity, there is always one 3-face that flows into the other across the common 2-face.
2. Build the 2-face graph of $K$.
   - Nodes correspond to 2-faces of $K$.
   - Edges correspond to two 2-faces that are adjacent to a common 3-face, with orientation determined by the Reeb flow direction on the common 3-face. At most one 2-face has a point that flows into the other across the common 3-face. If no flow occurs between the two 2-faces, we don't have an edge between them.
3. Attach data to the 2-face graph of $K$.
   - For each 2-face $i$, pick a projection onto $\mathbb{R}^2$. The image $A_i$ is a convex $2d$ polytope. We identify the 2-face with this polytope from now on, in an abuse of notation.
   - For each edge $i \to j$ with 3-face $F$ in between, compute the permissible subset $P_{i j} \subset A_i$ of points that the Reeb flow on $F$ carries into $A_j$, when we project the start and end point into $\mathbb{R}^2$ of course. We know that $P_{i j}$ is again a convex $2d$ polytope, and non-empty because the edge exists. Other points on $A_i \setminus P_{i j}$ flow into some other 2-face that exits the 3-face $F$.
   - We store the affine map $\psi: P_{i j} \to A_j$ that describes the Reeb flow on $F$.
   - We store the affine map $A_{\mathrm{inc}}: P_{i j} \to \mathbb{R}$ that describes the action increment when flowing from $A_i$ to $A_j$ along $F$.
   - We store the affine map $\rho_{\mathrm{inc}}: P_{i j} \to \mathbb{R}$ that describes the rotation increment when flowing from $A_i$ to $A_j$ along $F$. Here we define the rotation as $$ TODO $$
**Phase 2: Search for minimum action closed characteristics**
1. Every closed characteristic maps to a cycle in the 2-face graph of $K$. We can thus enumerate all graph cycles, check if any closed characteristic corresponds to the graph cycle, and what the minimum action of such a closed characteristic is. We then return the minimum over all graph cycles.
2. In order to be smarter about the enumeration, we skip graph cycles that cannot correspond to the minimum action closed characteristic. For this, we use the following observations, given some path $1, 2, \dots, k$ in the 2-face graph.
   - We can compute (iteratively even) the set of candidate starting points $C_{1, \dots, k} \subset A_1$. A point $p$ is a candidate if 
   a) its flow trajectory stays permissible: $$\psi_{1 \dots m}(p) \in A_m$$ for all $m = 1, \dots, k$, where $\psi_{1 \dots m}$ is the composition of all flow maps along the path from face $1$ to face $m$.
   b) its summed action increments stay below the current best upper bound on the minimal action: $$A_{\mathrm{inc}, 1 \dots m}(p) \leq A_{\mathrm{best}}$$ for all $m = 1, \dots, k$, where $A_{\mathrm{inc}, 1 \dots m} = A_{\mathrm{inc}, 1 \dots m-1} + A_{\mathrm{inc}, m-1, m} \circ \psi_{1 \dots m-1}$ is the accumulated action increment along the path from face $1$ to face $m$.
   c) its summed rotation increments stay below the upper bound $2$: $$\rho_{\mathrm{inc}, 1 \dots m}(p) \leq 2$$ for all $m = 1, \dots, k$, where $\rho_{\mathrm{inc}, 1 \dots m} = \rho_{\mathrm{inc}, 1 \dots m-1} + \rho_{\mathrm{inc}, m-1, m} \circ \psi_{1 \dots m-1}$ is the accumulated rotation increment along the path from face $1$ to face $m$.
   - Note here that the candidate set $C_{1, \dots, k}$ is again a convex $2d$ polytope, or empty.
   - If $C_{1, \dots, k}$ is empty, we can skip all graph cycles containing this path, since they don't permit any closed characteristic that also stays below the known smallest action and the rotation upper bound.
3. If we reach a graph cycle $1, 2, \dots, k, 1$, which has non-empty candidate set $C_{1, \dots, k, 1}$, we only have to check if any of the trajectories is closed. We look at the fixed points of the affine map $\psi_{1, \dots, k, 1}: \mathbb{R}^2 \to \mathbb{R}^2$. If no fixed point exists, or no fixed point lies in $C_{1, \dots, k, 1}$, we can skip this graph cycle. Otherwise, we pick the fixed point with minimum action, and use it as our new best candidate. By definition of $C_*$ we already know the action is below our current best upper bound, and the rotation is below $2$, and the trajectory is permissible. By fixed point property, the trajectory is closed.
4. It's useful to find a best candidate with low action, ideally even the true minimum action, early in the algorithm, so that we can skip more graph cycles later on. For this we should use these heuristics:
   - Short graph cycles may have lower action than long graph cycles
   - We can get a minimum action increment (that may be 0) from $A_{\mathrm{inc}, i j}$ on $P_{i j}$ for each edge $i \to j$. This gives us a non-negatively weighted directed graph, where the total weight of a cycle is a lower bound on the action of any closed characteristic mapping to this cycle. This is again a heuristic to prioritize graph cycles that may have lower action.
   - Similarly we can avoid exceeding the rotation budget by prioritizing low rotation increment edges.
5. The final algorithm looks like this:
   - Initialize the graph and its node and edge data
   - Initialize heuristics to find good candidates earlier
   - Initialize the upper bound $A_{\mathrm{best}}$ from some Viterbo-like theorem (iirc. sqrt(vol * 8) or something?)
   - Enumerate graph cycles:
     - We can use an outer loop over starting nodes, where later iterations then ban already considered starting nodes to avoid double work
     - We can incrementally build paths using DFS, and prioritize which edge to explore next using the heuristics
       - if an edge leads back to the start, check it first
       - greedily order edges by their minimum action increment and rotation increment
     - We discard any DFS branch if its path has empty candidate set
   - When reaching a closed graph cycle, solve the fixed point problem and update best candidate if the fixed point lies in the candidate set
6. Open Questions:
   - it may be useful to precompute $C_{i j k}$ edge-pairs, in particular if many are empty. This may speed up the DFS pruning, since it's a quick rejection check by doing a lookup based on the end of the path $\dots i j k$ rather than doing an ad-hoc recomputation whenever (i j k) pops up.

Remark: ah wait, i think the rotation increment is actually just a number we can read off $\psi_{i j}$. Oops. Yeah, this simplifies things for rotation a bit!

Implementation Details:
- We want to implement the algorithm in Rust, and as very clear & documented code, with debug instrumentation, and production performance.
- The main steps I think are
  - The 2-face graph
  - Optional: precompute bools $\emptyset = C_{i j k}$ for all edge pairs (i j k)
  - The enumeration of paths, using an outer loop over starting nodes, restricting to ignore already considered starting nodes, and picking using heuristics which edge to explore next in a DFS manner.
  - For each path, store data, and then compute on path extension the new data. reject based on the data.
  - concrete data structure suggestion:
    - path list of 2-face indices
    - subset of all 3-faces that any 2-face in the path flows into or out of
    - candidate set as a convex 2d polytope in half-space representation
    - accumulated rotation as number
    - accumulated action as an affine map (matrix 2x1, vector 1)
    - accumulated flow map as an affine map (matrix 2x2, vector 2)
  - if the candidate set is empty, reject
  - wrt 3-face constraint: the next edge must either flow into a 3-face not in the set yet, or flow into the 3-face that flows into the start 2-face, in which case the edge after that must close the loop i.e. go back to the start 2-face.
  - The update of the data structure on appending an edge k -> k+1 is:
    -  accumulate rotation $\rho_{1 \dots k+1} = \rho_{k, k+1} + \rho_{1 \dots k}$; reject if $>2$
    -  accumulate flow map $\psi_{1 \dots k+1} = \psi_{k, k+1} \circ \psi_{1 \dots k}$
    -  accumulate action $A_{1 \dots k+1} = A_{1 \dots k} + A_{\mathrm{inc}, k, k+1} \circ \psi_{1 \dots k}$
    -  intersect $C_{1, \dots, k+1}^* = C_{1, \dots, k} \cap \psi_{1 \dots k}^{-1}(P_{k, k+1})$; reject if empty
    -  intersect $C_{1, \dots, k+1} = C_{1, \dots, k+1}^* \cap \{z: \, A_{1, \dots, k+1}(z) \leq A_{\mathrm{best}}\}$; reject if empty
    -  update 3-face set
    -  update the path list
    -  indicate to force closure if the end 2-face flows into a 3-face that flows into the start 2-face
  - after the closed loop is calculated and not rejected:
    - solve the fixed point problem $\psi_{1, \dots, k, 1}(z) = z$
    - check if the fixed point lies in $C_{1, \dots, k, 1}$
    - if no, reject
    - if yes, store the new best candidate with action $A_{1, \dots, k, 1}(z)$
  - Final output is the best candidate found after the DFS enumeration is complete.
- For convex 2d polytopes, we can use half-space representation for intersections, for checking if a point lies in them, for representing a $A \leq A_{\mathrm{best}}$ constraint, and for checking emptiness (using a 2d LP solver). The pre-image under an affine map of a half-space representation is again a half-space representation, so all operations we need are supported.
- Unsure if pruning the half-space representation is useful or too costly.
- We use immutable data structures. The path may be stored by the enumeration algorithm and not be considered part of the data structure. The forced closure may also be done by the enumeration algorithm, not the extension step.
- We use our own nalgebra-based data structures, since we want control over how we represent affine maps (2x2 matrix + 2x1 vector) and half-spaces etc.
- At the end, the algorithm converts the stared fixpoint on the stored start 2-face into a piecewise linear closed characteristic on $\partial K$ by following the flow maps along the path and converting from $2d$ to $4d$ coordinates using the stored projections.
- We want to have extensive unit tests for all subcomponents.
- In debug, we do indexing checks, sanity checks, assertions, logging, etc.
- In production we optimize for speed, allow the CPU pipeline to work well, etc.
- We want to have benchmark data for typical polytopes we want to compute the EHZ capacity for, to see how well the implementation performs.
- We want to have profiling data to see where the bottlenecks are.
- We want in an even slower debug mode to get instrumentation data on how many paths are pruned etc. This helps us together with profiling data to decide where to optimize.

- It's okay to assume genericity of the polytope, concretely this implies that our fixed points never lie exactly on the boundary of the candidate set, and that there is a unique minimum cycle. Ofc numerical rounding can be a problem here, but I guess it'll be fine in practice bc we do not just guarantee genericness, but can even wiggle vertices a bit if needed. Potentially the way to go here is to have a warning mode, where we allow closed characteristics that go outside the candidate set by a small epsilon, to catch the minimum cycle if it ends up doing that due to rounding errors.
- Open Question: it may be true that we can just generally allow fixed points outside the candidate set, or even omit tracking the permissibility and only track rotation and action cutoffs. This would be implied by some theorem ala "the only fake characteristics (bc they don't actually lie on $\partial K$) that the algorithm then iterates over all have above-minimum action anyway, and will be discarded". This would simplify the algorithm, allow us to find fake-candidates with high action earlier, that still provide a true upper bound on the minimum action, resolves some trouble with numerical rounding errors, but on the other hand omits the pruning power of permissibility checks. Not sure if this is beneficial for performance, in addition to not being sure if it even is correct as an  algorithm.
- Potential Optimization: a quick check before computing permissibility whether the edge pair $k-1, k, k+1$ has empty $C_{i j k}$, to reject paths earlier. This boolean matrix may be precomputed in phase 1.

There are other algorithm ideas, such as caching subpaths of interest beyond length 2, but that seems less promising and more costly. So we won't go there likely, and be satisfied with what above algorithm does instead.