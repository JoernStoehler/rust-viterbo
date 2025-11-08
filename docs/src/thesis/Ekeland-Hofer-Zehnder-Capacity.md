<!--
Docs: docs/src/thesis/Ekeland-Hofer-Zehnder-Capacity.md
Ticket: 5ae1e6a6-5011-4693-8860-eeec4828cc0e
Notes for future editors:
 - Keep KaTeX-safe math (no \operatorname).
 - This section introduces the invariant and fixes conventions used
   by the algorithmic sections (see thesis/capacity-algorithm-*.md).
 - Prefer footnotes for citations to keep the flow readable.
-->

# The Ekeland–Hofer–Zehnder Capacity on Convex Polytopes in R^4

This section fixes conventions and recalls the precise definition and basic properties of the Ekeland–Hofer–Zehnder (EHZ) capacity in the 4‑dimensional Euclidean symplectic space. We then specialize to convex star‑shaped polytopes, describe the induced (generalized) Reeb dynamics on their boundary, and record structural facts we will use later when designing algorithms.

## Setting and Notation

We work on R^{4} with the standard symplectic form
$$
\omega_0 = dx_1\wedge dy_1 + dx_2\wedge dy_2,
$$
the standard complex structure \(J\) (so that \(\omega_0(u,v)=\langle Ju, v\rangle\)), and the standard Liouville 1‑form
$$
\lambda_0 = \tfrac12 \sum_{i=1}^{2} \big(x_i\,dy_i - y_i\,dx_i\big).
$$
Let \(K\subset \mathbb{R}^4\) be a compact, convex body that is star‑shaped with respect to the origin; write \(\Sigma=\partial K\). At smooth points of \(\Sigma\), the restriction \(\alpha=\lambda_0|_{\Sigma}\) is a contact form with \(d\alpha=\omega_0|_{\Sigma}\). The Reeb vector field \(R_\alpha\) is defined by \(\alpha(R_\alpha)=1\) and \(d\alpha(R_\alpha,\cdot)=0\). A closed characteristic on \(\Sigma\) is a closed orbit of \(R_\alpha\); for non‑smooth \(\Sigma\) (e.g. a polytope) we use generalized closed characteristics in the sense of convex/contact nonsmooth dynamics (the “combinatorial Reeb orbits” below), which agree with Reeb orbits after smoothing and preserve action and Conley–Zehnder index.[^CH21]

The action of a closed (generalized) characteristic \(\gamma\) is
$$
\mathcal{A}(\gamma)=\int_\gamma \lambda_0,
$$
which equals the \(\omega_0\)-area of any spanning disk when \(\gamma\) is contractible.

## Definition and Basic Properties

For convex domains the Ekeland–Hofer and Hofer–Zehnder capacities coincide, and their common value is denoted \(c_{EHZ}(K)\). It admits the following equivalent characterizations.

- Variational/Reeb characterization (convex case). The EHZ capacity is the minimal action of a closed (generalized) characteristic on \(\partial K\):
  $$
  c_{EHZ}(K) \;=\; \min\{\mathcal{A}(\gamma)\mid \gamma \text{ closed (generalized) characteristic on }\partial K\}.
  $$
  The infimum is attained.[^EH89][^CHLS07][^Irie19]

- Hofer–Zehnder Hamiltonian characterization. Let \(\mathcal{H}(K)\) be the set of autonomous “admissible” Hamiltonians supported in \(K\) with all non‑constant periodic orbits of period strictly larger than 1. Then
  $$
  c_{HZ}(K)=\sup\{\max H\mid H\in\mathcal{H}(K)\}.
  $$
  For convex \(K\), \(c_{HZ}(K)=c_{EHZ}(K)\).[^HZ94][^CHLS07]

As a symplectic capacity on the class of convex sets, \(c_{EHZ}\) satisfies the usual axioms:

- Monotonicity: if \(K\hookrightarrow K'\) symplectically, then \(c_{EHZ}(K)\le c_{EHZ}(K')\).
- Conformality: \(c_{EHZ}(aK)=a^2\,c_{EHZ}(K)\) for \(a>0\).
- Normalization: \(c_{EHZ}\big(B^{4}(r)\big)=\pi r^2\) and \(c_{EHZ}\big(Z^{4}(R)\big)=\pi R^2\), where \(B^{4}(r)\) is the Euclidean ball and \(Z^{4}(R)=\{(x,y)\in\mathbb{R}^{2}\times\mathbb{R}^{2}: \pi|y|^2<R^2\}\) the symplectic cylinder.[^HZ94][^CHLS07]

Remark (Minkowski billiards view). For convex Lagrangian products \(K\times T\subset\mathbb{R}^{2n}\), \(c_{EHZ}(K\times T)\) equals the minimal length of a closed \((K,T)\)–Minkowski billiard trajectory; this will be relevant for product examples.[^AAKO14][^Rudolf24]

## Reeb Dynamics on Polytopes in R^4

Let \(K\subset\mathbb{R}^4\) be a convex polytope that is star‑shaped with respect to the origin. The boundary \(\Sigma=\partial K\) is piecewise flat (a union of 3‑dimensional facets meeting along 2‑faces, 1‑faces, and 0‑faces). The classical Reeb vector field \(R_\alpha\) is defined and smooth on the relative interior of each facet; at non‑smooth points we use generalized characteristics (solutions to the Reeb differential inclusion), which can be defined combinatorially and arise as limits of Reeb orbits for smoothings of \(K\).[^CH21]

- Facets (3‑faces). If \(F\) is a facet with constant outer unit normal \(n_F\), then on \(\operatorname{relint}(F)\) the Reeb vector is parallel to \(J n_F\); thus the flow along \(F\) is linear, and generalized orbits are straight segments in direction \(J n_F\) with speed determined by the normalization \(\alpha(R_\alpha)=1\).[^CH21]

- Ridges/edges/vertices (2/1/0‑faces). At non‑smooth points, the Reeb vector field is not classically defined. Generalized characteristics cross these strata with velocity constrained to the “Reeb cone”, obtained by applying \(J\) to the outer normal cone of \(K\) at the point; in particular, an orbit may only spend measure‑zero time on these strata, and transitions satisfy a convex‑geometric reflection law.[^CH21]

Two structural facts are particularly important for computation and will be used later.

1) Minimal‑action orbit exists and is realized on \(\partial K\).  
For convex \(K\), \(c_{EHZ}(K)\) is achieved by a (generalized) closed characteristic. In 4D, recent results show the minimal‑action orbit on a convex three‑sphere bounds a disk‑like global surface of section; as a corollary, several capacities (including the cylindrical one) coincide with this minimal action. This further confirms attainment and strengthens the dynamical picture.[^AEK24]

2) “At most one visit per facet” for a minimizer on polytopes.  
There exists an action‑minimizing generalized closed characteristic whose intersection with the relative interior of each facet is empty or a single straight segment; in particular, it visits the interior of any given facet at most once. This yields a finite‑dimensional combinatorial/variational formula for \(c_{EHZ}(K)\) in terms of facet normals and positive weights subject to a balancing condition.[^HK19]

Edge segments force infinite rotation (exclude 1‑faces).  
Chaidez–Hutchings define a combinatorial rotation number \(\rho(\gamma)\) for generalized Reeb trajectories on 4D polytopes and prove that if \(\gamma\) contains a nontrivial segment contained in a 1‑face, then \(\rho(\gamma)=\infty\). Consequently, any closed orbit with finite Conley–Zehnder index (in particular, an EHZ minimizer in \(\mathbb{R}^4\)) cannot contain 1‑face segments; candidates may only run along facets and cross lower‑dimensional strata at isolated points.[^CH21]
## Index Information in Dimension Four

When \(\Sigma=\partial K\) is strictly convex and \(C^2\), the induced contact form on \(\Sigma\) is dynamically convex; all contractible closed Reeb orbits have Conley–Zehnder index at least \(3\). Under standard non‑degeneracy assumptions, an action‑minimizing simple orbit realizing \(c_{EHZ}(K)\) has Conley–Zehnder index \(3\) and plays a distinguished dynamical role (it bounds a global surface of section and controls sharp systolic inequalities).[^HWZ98][^ABHS18][^CH21]

We will rely on the lower bound “CZ\(\ge 3\)” property and, in non‑degenerate settings, on the fact that the minimizer has index \(3\), when pruning candidate orbits in our algorithms.

## CZ Index and Rotation for 2D Return Maps {#cz-rotation}

Setup. Let \((\Sigma,\alpha)\) be a contact hypersurface in \(\mathbb{R}^4\) with Reeb field \(R_\alpha\). Fix a local surface of section \(D\subset \Sigma\) transverse to \(R_\alpha\); the first‑return (first‑hit) map \(\Phi:D\to D\) preserves the area form \(d\alpha|_D\) and orientation. Along a closed orbit \(\gamma\) intersecting \(D\), the linearized return \(\mathrm{d}\Phi\) restricts to a path in \(\mathrm{Sp}(2)\) (after a choice of trivialization).

Rotation number. In a 2D trivialization, write the linearized return along \(\gamma\) as a path \(\Psi(t)\in \mathrm{Sp}(2)\), \(t\in[0,T]\). Lifting to the universal cover of \(\mathrm{Sp}(2)\) defines a real rotation number \(\rho(\gamma)\in \mathbb{R}_{\ge 0}\). For generic (non‑degenerate) elliptic closures, the endpoint is conjugate to a rotation by angle \(\theta\in(0,2\pi)\), and \(\rho = \theta/\pi \in (0,2)\).

Conley–Zehnder index in 2D. For such generic closures one has
\[
\mu_{\mathrm{CZ}}(\gamma) \;=\; \lceil \rho(\gamma)\rceil + \lfloor \rho(\gamma)\rfloor.
\]
In particular, an index‑\(3\) minimizer satisfies \(\rho(\gamma)\in(1,2)\).

Canonical charts and positivity. In our polytope setting, we fix once per ridge a canonical 2D chart determined by an orthonormal basis \((u_1,u_2)\) with \(\omega_0(u_1,u_2)>0\). The per‑edge first‑hit maps between ridges are orientation‑preserving on admissible domains, so the per‑edge rotation increments are non‑negative and \(\rho\) accumulates additively along cycles.

Numerics (implementation note). We read \(\rho\) from the orthogonal polar factor of the 2×2 linear part of the charted map (principal angle divided by \(\pi\)). This is invariant under uniform scalings of the chart and does not require the Euclidean chart to be \(d\alpha\)–unit‑normalized.

## Normalization, Systolic Ratio, and Current Status

We use the normalization above, so for any convex \(K\subset\mathbb{R}^4\) we define the symplectic systolic ratio
$$
sys(K)=\frac{c_{EHZ}(K)^2}{2\,vol(K)}.
$$
Viterbo’s 2000 conjecture asked whether \(\operatorname{sys}(K)\le 1\) for all convex \(K\). This has been disproved very recently by Haim‑Kislev and Ostrover (accepted October 8, 2025, Annals of Mathematics), which in particular shows that normalized symplectic capacities need not coincide on convex domains.[^HKO25]

## What We Use Later

- The facet‑linearity of the Reeb flow and the Reeb‑cone transition rule at non‑smooth strata (to build discrete search spaces of candidate orbits).[^CH21]
- The “at most one visit per facet” structure and facet‑weight formula (to derive compact finite programs).[^HK19]
- The index constraints in 4D (to filter candidates by Conley–Zehnder index).[^HWZ98][^ABHS18]

Further algorithmic details are in Oriented‑Edge Graph Algorithm (see `docs/src/thesis/capacity-algorithm-oriented-edge-graph.md`) and the linear/variational formulations (`docs/src/thesis/capacity-algorithm-linear-program.md`).

---

Footnotes / references (selection; see also `thesis/bibliography.md`):

[^CH21]: Chaidez, J.; Hutchings, M. Computing Reeb dynamics on four‑dimensional convex polytopes. J. Comput. Dyn. 8(4):403–445, 2021; arXiv:2008.10111.
[^CHLS07]: Cieliebak, K.; Hofer, H.; Latschev, J.; Schlenk, F. Quantitative symplectic geometry. MSRI Publ. 54 (2007).
[^EH89]: Ekeland, I.; Hofer, H. Symplectic topology and Hamiltonian dynamics I–II. Math. Z. 200 (1989), 203 (1990).
[^HZ94]: Hofer, H.; Zehnder, E. Symplectic Invariants and Hamiltonian Dynamics. Birkhäuser, 1994.
[^Irie19]: Irie, K. Symplectic homology of fiberwise convex sets and homology of loop spaces. arXiv:1907.09749 (2019→, with updates).
[^HK19]: Haim‑Kislev, P. On the symplectic size of convex polytopes. Geom. Funct. Anal. 29 (2019), 440–463.
[^AEK24]: Abbondandolo, A.; Edtmair, O.; Kang, J. On closed characteristics of minimal action on a convex three‑sphere. preprint (2024).
[^AAKO14]: Artstein‑Avidan, S.; Karasev, R.; Ostrover, Y. From symplectic measurements to the Mahler conjecture. Duke Math. J. 163 (2014), 2003–2022.
[^Rudolf24]: Rudolf, D. The Minkowski billiard characterization of the EHZ‑capacity of convex Lagrangian products. J. Dyn. Diff. Eq. (2024); arXiv:2203.01718.
[^HWZ98]: Hofer, H.; Wysocki, K.; Zehnder, E. The dynamics on three‑dimensional strictly convex energy surfaces. Ann. Math. 148 (1998).
[^ABHS18]: Abbondandolo, A.; Bramham, B.; Hryniewicz, U.; Salomão, P. Sharp systolic inequalities for Reeb flows on S^3. Invent. Math. 211 (2018); and Systolic ratio, index of closed orbits and convexity for tight contact forms on S^3, Compos. Math. 154 (2018).
[^HKO25]: Haim‑Kislev, P.; Ostrover, Y. A counterexample to Viterbo’s Conjecture. Annals of Mathematics (accepted Oct 8, 2025).

---

Deviations and clarifications for review

- “No Reeb flow on 2‑faces/1‑faces”: At non‑smooth strata the classical Reeb vector field is undefined. I replaced this with the standard “Reeb cone”/generalized‑characteristic language and cited the polytope‑specific treatment (Chaidez–Hutchings). This yields the same computational consequences (piecewise linear motion with instantaneous transitions) without overstating non‑existence.  
- “Direction \(J(a n_1 + b n_2)\) on 1‑faces”: Rather than a specific formula, I stated the precise constraint “velocity lies in \(J\) of the outer normal cone,” which subsumes the two‑facet case and generalizes correctly at higher‑valence strata.  
- “Segments on 1‑faces have infinite rotation”: Now asserted and cited to Chaidez–Hutchings; see “Edge segments force infinite rotation (exclude 1‑faces)”.  
- “Minimum is attained”: Strengthened and cited via symplectic‑homology/smoothing arguments (and a 2024 result showing the minimizer bounds a global surface of section in 4D).  
- “Visits every 3‑face at most once”: Included as an existence statement with a citation to Haim‑Kislev (2019).  
- Viterbo conjecture: Updated to reflect the 2024–2025 counterexample and included the Annals acceptance dates for clarity.  
- Scope: I deferred algorithmic details to the separate algorithm sections and kept this file focused on definitions/properties, as requested.
