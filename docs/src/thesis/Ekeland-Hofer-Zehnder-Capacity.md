<!-- Author: Codex -->

TODO: replace with a concise, specific, correct, and formal definition of
- our setting: R^4, standard symplectic form, standard Liouville form, convex star-shaped non-degenerate polytopes
- closed characteristics on the boundary of the polytope
- the action functional
- the minimum action == EHZ capacity
- theorems with citations. Here's what I recall, sadly without source and rather informally stated:
  - the minimum is attained
  - A_min = c_EHZ is a symplectic capacity and fulfills the axioms (we can list them)
  - the action-minimizing closed characteristic is a periodic orbit of the Reeb flow on the boundary of the polytope
  - on 3-faces, the Reeb flow is linear in the direction $J n$, where n is the outer normal vector to the face
  - on 2-faces, no Reeb flow exists, so the minimum orbit must cross the 2-faces, not flow along them
  - on 1-faces, a Reeb flow exists in some direction J (a n_1 + b n_2) iirc, where a,b are such that the direction is in the 1-face, not sure though if that formula is right; the orbit can enter and leave the 1-face arbitarily
  - on 0-faces, no Reeb flow exists, but they only get crossed anyway
  - hence any orbit is a concatenation of line segments in the 3-faces and 1-faces, crossing 2-faces and 0-faces at single points
  - there's some rotation (Conley-Zehnder index) we can define (i don't recall rn the definition)
  - the action integral and the rotation integral both increase along any combinatorical Reeb flow trajectory
  - the *minimum* action orbit has CZ index 3, i.e. rotation $1 < \rho(\gamma) < 2$. 
  - Segments on 1-faces have infinite rotation, so the minimum orbit cannot contain any 1-face segments.
  - Conjecture: in the generic case, the minimum orbit doesn't cross 1-faces either, so it entirely consists of segments in 3-faces, crossing 2-faces at single points.
  - The Viterbo Conjecture (which is false) states that $sys(K) := c_EHZ(K)^2 / (2 vol(K)) <= 1$
  - There's a known constant C s.t. we know that $c_EHZ(K)^2 <= C vol(K)$ at least, so we have an upper bound readily available for the capacity.
  - *IIUC* there's a theorem that the *minimum* action orbit visits every 3-face at most once. This is a strong statement and very useful, so please look up the source and confirm or deny it.

We end this document before we start describing algorithms to compute the EHZ capacity and the minimum action orbit. Other documents will cover different algorithms.

References:
- See the literature list thesis/bibliography.md for potentially relevant papers and books.
