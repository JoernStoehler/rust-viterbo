<!-- Author: Codex & Jörn -->

<!-- Ticket: Please add the VK ticket UUID here once available. -->

<!-- Docs: docs/src/thesis/Ekeland-Hofer-Zehnder-Capacity.md -->

# Oriented-Edge Graph Algorithm for c_EHZ in R^4

<!-- Why: This document specifies a high-level, implementation-ready algorithm to compute the Ekeland–Hofer–Zehnder (EHZ) capacity of a convex, star-shaped, non-degenerate polytope in R^4 by reducing the search for action-minimizing Reeb orbits to a directed cycle search on a 2-face graph with polyhedral feasibility checks. We progressively disclose formal definitions, maps, and constraints, then the search strategy and implementation notes. -->

<!-- Scope: High-level algorithm and the precise per-edge maps and constraints we need. Low-level performance choices are collected at the end. Mathematical background, including precise definitions of action, Reeb dynamics, and rotation/CZ index, is in the EHZ capacity document. -->

## Goal
- Input:
  - Half-space description of a convex, star-shaped, non-degenerate polytope $K \subset \mathbb{R}^4$: $K=\{x\in \mathbb{R}^4:\langle n_f,x\rangle\le b_f\ \forall f\in \mathcal{F}_3\}$, where each facet (3-face) $f$ has outward unit normal $n_f$ and support constant $b_f>0$.
  - Optionally, vertices for convenience and validation.
- Output:
  - The EHZ capacity $c_{\mathrm{EHZ}}(K)$.
  - An action-minimizing closed characteristic $\gamma\subset\partial K$ represented combinatorially by a directed cycle of 2-faces and a fixed point in the induced affine map on the start 2-face, together with an explicit piecewise-linear lift to $\partial K$.

<!-- Comment: “non-degenerate” here is the generic position assumption we actually use algorithmically: no Reeb direction is parallel to a ridge; no two exit times tie on a positive-measure set; fixed points do not lie on candidate-set boundaries. Precise statements in Assumptions. -->

## Setting and Assumptions
- Space and forms:
  - Standard symplectic form $\omega_0$ and Liouville form $\lambda_0$ on $\mathbb{R}^4$.
  - Fix the standard complex structure $J$ so that $\omega_0(u,v)=\langle Ju, v\rangle$ and $J^\top J=I$, $J^\top=-J$.
- Symplectic polytope hypothesis:
  - No 2-face is Lagrangian: for every 2-face $F$, the restriction $\omega_0|_{TF}\ne 0$. This matches the “symplectic polytope” condition in the Chaidez–Hutchings framework and ensures well-posed combinatorial Reeb dynamics across ridges.
- Facets and Reeb directions:
  - For each facet $f\in \mathcal{F}_3$, with plane $H_f=\{x:\langle n_f,x\rangle=b_f\}$, trajectories of the Reeb flow on $H_f\cap\partial K$ are straight segments parallel to $v_f:=J n_f$ (speed may vary; directions are constant).
  - We only need directions to get exit points; actions are integrals of $\lambda_0$ along these straight segments.
- Genericity/non-degeneracy assumptions (used for correctness and robust numerics):
  1) For each facet $f$, and each ridge $r\subset f$ with co-facet $g\ne f$, we have $\langle v_f, n_g\rangle\ne 0$.  
  2) For a fixed $f$, in the region where a particular co-facet $g$ is the first one hit along $v_f$, that co-facet is uniquely first (no ties on a set of positive measure).  
  3) Action-minimizing cycles do not involve segments on 1-faces (rotation blow-up); crossings of 2-faces occur at single points.  
  <!-- Docs: docs/src/thesis/Ekeland-Hofer-Zehnder-Capacity.md#setting -->
  <!-- Comment: (1)-(3) match the “generic” case we intend to handle first; degenerate tie-breaking and 1-face handling can be added later. -->
  <!-- Comment: The non-Lagrangian 2-face hypothesis lines up with our ridge-crossing model and with CH’s notion of symplectic polytopes. -->

## Face Graphs
- 3-face digraph:
  - Nodes: facets $f\in \mathcal{F}_3$.
  - Oriented edges $f\xrightarrow{r} f'$ whenever facets $f\ne f'$ share a ridge $r$ and the direction $v_f$ points from a neighborhood of $r$ in $f$ into the interior of $f'$ across $r$ (equivalently: the first exit from $f$ along $v_f$ near points on $r$ is $f'$).
  - This orientation is well-defined by convexity and the genericity assumptions.
- 2-face digraph (the main search graph):
  - Nodes: ridges $i\in \mathcal{F}_2$. Each ridge $i$ is the intersection of two distinct facets $f(i)$ and $g(i)$.
  - Oriented edges: $i\to j$ labeled by the facet $F$ if $i,j\subset F$ and the flow along $v_F$ from points of $i$ first exits $F$ through $j$.
  - Multiple outgoing edges from a ridge within a common facet are possible; absent edges correspond to “no point flows $i\to j$ first”.
  <!-- Comment: This is the “oriented-edge” viewpoint: we travel along facets, cross ridges at single points. -->
  - Orientation convention (decision): for every ridge $i$, fix the chart $U_i$ to be the canonical one induced by $\omega_0$ (choose an orthonormal basis $(u_1,u_2)$ of the face plane with $\omega_0(u_1,u_2)>0$). This pins the sign of rotation angles extracted from $D\psi_{ij}$.

## Notation Recap
- Geometry: $\omega_0$ (standard symplectic form), $\lambda_0$ (Liouville), $J$ (standard complex structure) on $\mathbb{R}^4$.
- Facets: for each $F\in\mathcal{F}_3$, outward unit normal $n_F$ and support $b_F>0$, plane $H_F=\{x:\langle n_F,x\rangle=b_F\}$, Reeb direction $v_F:=J n_F$.
- Ridges: $i\in\mathcal{F}_2$ with affine plane $R_i\subset H_F$. Charts $\pi_i:R_i\to\mathbb{R}^2$ define $A_i:=\pi_i(i)$.
- Per-edge quantities along $i\xrightarrow{F}j$:
  - Exit time $\tau_{ij}(x)$; affine on regions of constant first exit.
  - Affine map $\psi_{ij}:\operatorname{dom}\psi_{ij}\to A_j$, where $\operatorname{dom}\psi_{ij}\subset A_i$ and $\operatorname{im}\psi_{ij}\subset A_j$ are convex polygons.
  - Action increment $A_{ij}(x)=\tfrac{b_F}{2}\,\tau_{ij}(x)$ (affine on $\operatorname{dom}\psi_{ij}$).
  - Rotation increment $\rho_{ij}\ge 0$ (we use polar angle of $D\psi_{ij}$; see Rotation).
<!-- review: feel free to delete after review if redundant with later sections. -->

## Algorithm Summary (push-forward only)
- Maintain, at the current ridge, a candidate polygon $C\subset A_{i_k}$, an affine action $A:C\to\mathbb{R}$, a scalar rotation $\rho$, and an optional composed map $\Psi$ to the start chart.
- To extend along an edge $i_k\xrightarrow{F} i_{k+1}$:
  - Gate at $i_k$: intersect $C$ with $\operatorname{dom}\psi_{i_ki_{k+1}}\subset A_{i_k}$ (points that flow first to $i_{k+1}$ across $F$).
  - Push-forward candidates: $C'=\psi_{i_ki_{k+1}}\!\bigl(C\cap \operatorname{dom}\psi_{i_ki_{k+1}}\bigr)\subset \operatorname{im}\psi_{i_ki_{k+1}}\subset A_{i_{k+1}}$.
  - Update action $A'$ via composition with $\psi^{-1}$ and add the per-edge increment; prune by $A'(z)\le A_{\mathrm{best}}$; update $\rho'=\rho+\rho_{i_ki_{k+1}}\le 2$.
  - Repeat; on returning to the start ridge, solve the fixed-point equation $\Psi(z)=z$ within $C$ and update the incumbent.
- Enforce “simple loop” pruning: never revisit a facet (Haim–Kislev 2017).

## Per-edge Maps and Polyhedral Domains
Fix an oriented edge $i\xrightarrow{F} j$ in the 2-face graph, with $F\in \mathcal{F}_3$, $i,j\subset F$. Let $G(j,F)$ denote the co-facet that, together with $F$, defines $j$.

- Exit-time formula on $F$:
  - For $x\in H_F$ near $i$, the first time the straight line $x + t\,v_F$ hits the plane $H_{G(j,F)}$ is
    $$\tau_{ij}(x)\;=\;\frac{b_{G(j,F)}-\langle n_{G(j,F)},x\rangle}{\langle n_{G(j,F)}, v_F\rangle},\quad \text{with }\ \tau_{ij}(x)>0.$$
  - The condition that $j$ is indeed first exit among all co-facets $k\subset F$ is
    $$\tau_{ij}(x)\le \tau_{ik}(x)\quad\text{for all admissible }k,$$
    where “admissible” means $\langle n_k,v_F\rangle>0$ (the ray intersects $H_k$ forward in time) and $x+t\,v_F$ stays in $F$ for $t\in[0,\tau_{ik}(x)]$.
    These inequalities are linear in $x$ after multiplying by the (fixed) denominators’ signs.
  - Explicit half-space description of the domain $\operatorname{dom}\psi_{ij}$:
    - Let $\mathcal{K}_F$ be the set of co-facets $k$ of $F$ with $\langle n_k,v_F\rangle\ne 0$. Define $\sigma_k:=\operatorname{sign}\langle n_k,v_F\rangle$.
    - For $x\in H_F$, the comparison $\tau_{ij}(x)\le \tau_{ik}(x)$ is equivalent to
      $$
      \sigma_k\bigl(b_{G(j,F)}-\langle n_{G(j,F)},x\rangle\bigr)\,\langle n_k,v_F\rangle
      \;\le\;
      \sigma_k\bigl(b_k-\langle n_k,x\rangle\bigr)\,\langle n_{G(j,F)},v_F\rangle.
      $$
    - Combine these with $x\in i$ and $\tau_{ij}(x)>0$ (a single linear inequality after sign normalization). Projecting by $\pi_i$ yields $\operatorname{dom}\psi_{ij}\subset A_i$ as a convex polygon in half-space form.
- Domains and images:
  - Domain (in $A_i$): $\operatorname{dom}\psi_{ij}\subset A_i$ consists of ridge points that flow first to ridge $j$ across facet $F$ (convex polygon).
  - Image (in $A_j$): $\operatorname{im}\psi_{ij}=\psi_{ij}(\operatorname{dom}\psi_{ij})\subset A_j$ (convex polygon).
- Exit point and affine map:
  - Exit point in $F$: $x' = x + \tau_{ij}(x)\, v_F$, affine in $x$ on the region where $j$ is first exit.
  - Let $R_i$ and $R_j$ be the affine 2-planes containing ridges $i$ and $j$. Choose fixed linear charts (projections) $\pi_i:R_i\to \mathbb{R}^2$ and $\pi_j:R_j\to \mathbb{R}^2$ for every ridge; identify $A_i:=\pi_i(i)\subset\mathbb{R}^2$.
  - Define the per-edge affine map
    $$\psi_{ij}:\ \operatorname{dom}\psi_{ij}\ \to\ A_j,\qquad \psi_{ij}(\pi_i(x))\;=\;\pi_j\bigl(x+\tau_{ij}(x)\,v_F\bigr),$$
    with $\operatorname{dom}\psi_{ij}\subset A_i$ as above. By convexity and genericity, $\operatorname{dom}\psi_{ij}$ is a convex polygon (possibly empty), $\psi_{ij}$ is affine on it, and $\operatorname{im}\psi_{ij}$ is convex in $A_j$.
  <!-- Comment: We explicitly avoid parameterization of the Reeb vector field. Straight-line geometry suffices to locate exits and compute actions. -->
<!-- review: confirm chart orientation convention below works for rotation sign consistency. -->

Symbol map (equations above)
- $\psi_{ij}$: push‑forward map (code: `EdgeData.map_ij`).
- $\operatorname{dom}\psi_{ij}\subset A_i$: domain polygon in ridge $i$ (code: `EdgeData.dom_in`).
- $\operatorname{im}\psi_{ij}\subset A_j$: image polygon in ridge $j$ (code: `EdgeData.img_out`).
- $\tau_{ij}$: first‑exit time on facet $F$ (affine on the region where $j$ is first exit).
- $A_{ij}$: action increment (code: `EdgeData.action_inc`).
- $\rho_{ij}$: rotation increment from the polar angle of $D\psi_{ij}$ (code: `EdgeData.rotation_inc`).
- $U_i,U_j$: ridge charts (code: `Ridge.chart_u`; left‑inverse on the plane: `Ridge.chart_ut`).
- $v_F=J n_F$: facet Reeb direction (code: `geom4::reeb_on_facets`).
- $A_i$: ridge polygon in chart $i$ (code: `Ridge.poly`).
<!-- note: reviewers — this symbol map keeps equations compact while aligning with code identifiers for agents. -->

### Worked Example (axis‑aligned facet in the 4D cube)
Consider $K=[-1,1]^4$ in coordinates $(x_1,x_2,y_1,y_2)$ with the standard $J$ (so $v=J n$). Take the facet
$F=\{x_1=1\}$ with outward normal $n_F=e_{x_1}$ and $b_F=1$, hence $v_F=J n_F = e_{y_1}$.

- Choose ridges $i = F\cap\{y_1=-1\}$ and $j = F\cap\{y_1=+1\}$. The co‑facet for $j$ is $H_{G(j,F)}:\{y_1=1\}$ with $n_{G(j,F)}=e_{y_1}$, $b_{G(j,F)}=1$.
- Then $d_j=\langle n_{G(j,F)}, v_F\rangle = \langle e_{y_1}, e_{y_1}\rangle = 1 > 0$, and
  $$\tau_{ij}(x)=\frac{b_{G(j,F)}-\langle n_{G(j,F)},x\rangle}{d_j} = 1 - y_1.$$
  All other co‑facets $k$ with $\langle n_k, v_F\rangle\le 0$ are inadmissible, so $j$ is uniquely first exit.
- Charts: the ridge planes $R_i=R_j=\{x_1=\pm 1,\ y_1=\mp 1\}$ are spanned by the $(x_2,y_2)$ axes, so we may take $\pi_i,\pi_j$ as identity on $(x_2,y_2)$. Thus $U_iU_j^\top=I$ and the push‑forward map $\psi_{ij}$ is the identity on $(x_2,y_2)$.
- Rotation increment: $D\psi_{ij}=I_2 \Rightarrow \rho_{ij}=0$.
- Action increment: $A_{ij}(x)=\tfrac{b_F}{2}\tau_{ij}(x)=\tfrac{1}{2}(1 - y_1)$, which is affine and, in the chart, constant with respect to $(x_2,y_2)$.
<!-- note: reviewers — this concrete example shows the formulas reduce correctly to identity ψ and zero ρ in a simple axis‑aligned setting. -->

## Action Increment per Edge (explicit affine form)
For $x\in i$ that flows to $j$ across facet $F$, the action increment along the segment is
$$
A_{ij}(x)\;=\;\int_0^{\tau_{ij}(x)} \lambda_0\bigl(\dot \gamma(t)\bigr)\,dt
\quad\text{with}\ \gamma(t)=x+t\,v_F.
$$
Using $\lambda_0(\dot\gamma)=\tfrac{1}{2}\langle J\gamma,\dot\gamma\rangle$ and $Jv_F=J(Jn_F)=-n_F$, we obtain the identity
$$
A_{ij}(x)\;=\;\frac{1}{2}\,\langle x, n_F\rangle\ \tau_{ij}(x)\ =\ \frac{b_F}{2}\ \tau_{ij}(x),
$$
since $\langle x,n_F\rangle=b_F$ on the facet plane $H_F$. Therefore $A_{ij}$ is affine in $x$ on $\operatorname{dom}\psi_{ij}$ (because $\tau_{ij}$ is affine there).
In ridge coordinates, we treat $A_{ij}$ as an affine functional on $\operatorname{dom}\psi_{ij}\subset A_i$.
<!-- Comment: This formula is independent of the speed choice for the Reeb flow; only directions matter. -->

### Orientation and Chart Conventions
- For each ridge $i$, choose an orthonormal basis $(e_1,e_2)$ of $R_i$ so that the restriction $\omega_0|_{R_i}$ corresponds to the positive area form $dx\wedge dy$ under $\pi_i(e_1)=e_x$, $\pi_i(e_2)=e_y$.
- This fixes the sign of rotation angles extracted from $D\psi_{ij}$ unambiguously across ridges.
<!-- review: confirm this convention matches your preferred trivialization style. -->

## Rotation normalization and cutoff
(See the dedicated section below for the precise definition and guards; this subsection is intentionally concise.)
## Search Over Directed Cycles (push-forward variant)
We now describe the core enumeration and pruning in the 2-face digraph using push-forwards (no pull-backs of polytopes).

Notation for a path $p=(i_1\xrightarrow{} i_2\xrightarrow{}\cdots\xrightarrow{} i_k)$:
- Candidate set (current ridge coordinates): $C_p\subset A_{i_k}$, a convex polygon.
- Accumulated action (affine functional on $A_{i_k}$): $A_p(z)$.
- Accumulated rotation (scalar): $\rho_p$.
- Accumulated map to the start chart: $\Psi_p := \psi_{i_1i_2}\circ\cdots\circ \psi_{i_{k-1}i_k}$ when needed to close a cycle.

Initialization at a start ridge $i_1$:
- $C_{(i_1)} := A_{i_1}$,
- $A_{(i_1)}(z) := 0$,
- $\rho_{(i_1)} := 0$,
- $\Psi_{(i_1)} := \mathrm{Id}$.

Path extension by an edge $i_k \xrightarrow{} i_{k+1}$:
1) Push-forward candidates: $C' := \psi_{i_ki_{k+1}}( C_p \cap \operatorname{dom}\psi_{i_ki_{k+1}} ) \subset \operatorname{im}\psi_{i_ki_{k+1}}\subset A_{i_{k+1}}$. Reject if empty.  
2) Update action: $A'(z) := A_p\bigl(\psi_{i_ki_{k+1}}^{-1}(z)\bigr) + A_{i_ki_{k+1}}\bigl(\psi_{i_ki_{k+1}}^{-1}(z)\bigr)$ on $C'$.  
3) Prune by action budget: intersect $C' \leftarrow C' \cap \{z:\ A'(z)\le A_{\mathrm{best}}\}$. Reject if empty.  
4) Update rotation: $\rho' := \rho_p + \rho_{i_ki_{k+1}}$. Reject if $\rho'>2$.  
5) Update map: $\Psi' := \Psi_p\circ \psi_{i_ki_{k+1}}$ if we plan to close at $i_1$ soon; otherwise we can maintain only the last few factors and recompute on demand.  
6) Continue DFS with the new state $(C',A',\rho',\Psi')$.

Closing a cycle at $i_1$:
- When $i_{k+1}=i_1$, solve the fixed-point problem $\Psi_p(z)=z$ in $A_{i_1}$; keep any fixed point $z_\star\in C_p$; set $A_\star:=A_p(z_\star)$; if $A_\star<A_{\mathrm{best}}$, update the incumbent $(A_{\mathrm{best}}, \text{cycle}, z_\star)$.
- If no eligible fixed point exists in $C_p$, discard the cycle.

Heuristics and ordering:
- Prefer edges with small lower bounds on $A_{ij}$ (minimize the affine functional on $\operatorname{dom}\psi_{ij}$ via a tiny LP); break ties by smaller $\rho_{ij}$.
- Prefer short cycles first; try immediate back-edges that close at the start ridge early.
- Maintain a visited set of start ridges to avoid duplicate work; optionally restrict to simple cycles unless we decide otherwise (see Open Questions).

### Fixed-point solver (deterministic and robust)
- Write $\Psi_p(z)=Mz+t$ in the start chart. Solve $(I-M)z=t$:
  - If $\det(I-M)\ne 0$: unique fixed point $z_\star=(I-M)^{-1}t$, accept if $z_\star\in C_p$.
  - If $\det(I-M)=0$: use SVD to check feasibility; the fixed-point set is empty or an affine line. Intersect with $C_p$ and minimize $A_p(z)$ over this intersection (1D LP). Reject if empty.
- Tolerances: treat $|\det(I-M)|<\varepsilon$ as degenerate; enforce feasibility and membership with a consistent tolerance shared with tie-breaking $\varepsilon_\tau$.
<!-- note: reviewers — we keep equations compact and map symbols to code below. -->

Symbol map (fixed‑point and tolerances)
- $M,t$: entries of the composed affine map $\Psi_p$ (code: `State.phi_start_to_current`).
- $z,z_\star$: points in the start ridge chart (code: `Vec2`; returned by `dfs_solve_with_fp` helpers).
- $C_p$: candidate polygon at the start ridge (code: `State.candidate` on closure).
- $A_p$: accumulated action on the start chart (code: `State.action`).
- $\varepsilon_{\det}$: determinant threshold (code: `GeomCfg.eps_det`).
- $\varepsilon_{\mathrm{feas}}$: feasibility/membership slack (code: `GeomCfg.eps_feas`).
- $\varepsilon_{\tau}$: tie‑breaking and admissibility slack (code: `GeomCfg.eps_tau`).
<!-- note: agents — fixed_point_in_poly implements the 2D/1D branches with these exact eps values. -->

- Implementation guardrails:
  - `fixed_point_in_poly` handles both branches and switches to a 1D LP when $(I-M)$ is nearly singular so that we never rely on unstable matrix inverses.
  - `rotation_angle` returns `None` only for orientation-reversing maps; canonical chart construction rules those out, so failures signal numerical bugs instead of algorithmic cases.

## Choosing Budgets and Bounds
- Upper bound $A_{\mathrm{best}}$:
  - Practical: use that $K\subset B_R$ implies $c_{\mathrm{EHZ}}(K)\le c_{\mathrm{EHZ}}(B_R)=\pi R^2$. Compute $R$ from vertices or support data for a quick initial bound.
  - Tighter: use the volume-capacity inequality documented in `Docs: docs/src/thesis/Ekeland-Hofer-Zehnder-Capacity.md#volume-upper-bounds` once we finalize the preferred constant $C_{\mathrm{vol}}$. Reference that doc (not this page) whenever we update $C_{\mathrm{vol}}$ so the inequality stays centralized.
- Lower bound for progress reporting: $c_{\mathrm{EHZ}}(K)\ge \pi r^2$ if $B_r\subset K$ (inradius).
<!-- review: confirm default A_best choice (πR^2) until we pin the best constant C. -->

## Correctness Sketch (informal)
1) Every closed characteristic in the generic polytope case intersects ridges at isolated points and travels linearly on facets parallel to $v_f$.  
2) Such a trajectory maps to a directed cycle in the 2-face digraph; the per-edge maps and domains capture exactly the “first exit” geometry.  
3) The action along a cycle equals the sum of per-edge increments evaluated at the unique fixed point $z_\star$ of the composed affine map in the start chart.  
4) Minimizing action over all closed characteristics is thus equivalent to minimizing over all directed cycles and their fixed points.  
5) The push-forward pruning is sound: removing paths with empty candidate sets or with $A>A_{\mathrm{best}}$ or $\rho>2$ cannot delete the true minimizer.  
<!-- Comment: We will formalize this and connect to CZ index in the EHZ background document. -->

### Orientation lemma (canonical charts)
Lemma. Let $i\subset F$ and $j\subset G$ be ridges such that $\omega_0|_{Ti}\ne 0$ and $\omega_0|_{Tj}\ne 0$. With our canonical 2‑face charts $U_i,U_j$ (orthonormal bases oriented by $\omega_0(u_1,u_2)>0$), the Reeb first‑hit map $\psi_{ij}:U_i(i)\to U_j(j)$ is orientation‑preserving: $\det D\psi_{ij}>0$ wherever defined.

Proof (sketch). On each facet $F$, $\alpha:=\lambda_0|_F$ is a contact form and $R$ the Reeb vector field satisfies $\mathcal{L}_R\alpha=i_R d\alpha+d(\alpha(R))=0$, so the Reeb flow preserves both $\alpha$ and $d\alpha$.[^PreserveAlpha] A local surface of section $D\subset F$ transverse to $R$ inherits the positive area form $d\alpha|_D$; the Poincaré first‑hit map preserves $d\alpha|_D$ and hence orientation on $D$.[^ReturnArea] In our chart $U_i$, $d\alpha|_{Ti}=\omega_0|_{Ti}$ is $c\,dy_1\wedge dy_2$ with $c>0$ by construction; therefore $\det D\psi_{ij}>0$ in $y$‑coordinates. The same holds at $j$, so $\psi_{ij}$ preserves the canonical $\mathbb{R}^2$ orientation.

Remark. If a ridge were Lagrangian ($\omega_0|_{Ti}=0$), it would not define a transverse section and no return map is available. Our genericity excludes this case and matches the combinatorial Reeb model on 4D polytopes.[^CH]

### Rotation normalization and cutoff
- Data: For an oriented edge $i\to j$ inside facet $F$, the affine transition on ridge charts is $y_j = M_{ij}\,y_i + t_{ij}$ (Section “Per‑edge maps”). In canonical charts, $M_{ij}\in \mathrm{GL}^+(2)$ is orientation‑preserving and area‑preserving up to rounding.
- Definition (principal angle). Write the polar decomposition $M_{ij}=R_{ij}S_{ij}$ with $R_{ij}\in \mathrm{SO}(2)$ and $S_{ij}\succ 0$. Define
  \[
    \operatorname{rot}(M_{ij}) := \arg(R_{ij}) \in [0,\pi],\qquad
    \rho_{ij} := \frac{\operatorname{rot}(M_{ij})}{\pi}\in[0,1].
  \]
  Numerically we compute $R_{ij}$ via SVD (or a symmetric polar factor) and take $\operatorname{rot}(M_{ij})=\operatorname{atan2}((R_{ij})_{12},(R_{ij})_{11})$; we assert $\det(R_{ij})>0$ (otherwise the edge is invalid in our model).
- Alternatives.
  - Trace formula for orthogonal matrices: if $M_{ij}$ were itself orthogonal then $\operatorname{rot}=\arccos(\tfrac12\operatorname{tr}(M_{ij}))$. In general $M_{ij}$ is not orthogonal, so we apply the trace to $R_{ij}$, not to $M_{ij}$: $\operatorname{rot}=\arccos(\tfrac12\operatorname{tr}(R_{ij}))$. We prefer the polar/SVD route for robustness.
  - Eigen‑angle (elliptic check): in exact arithmetic for $M\in \mathrm{SL}(2,\mathbb{R})$ elliptic, $|\operatorname{tr}M|<2$ and the eigenvalues are $e^{\pm i\theta}$ with $\theta\in(0,\pi)$, but extracting $\theta$ reliably still benefits from the polar route in floating point.
- Guards and tolerances. We clamp arguments to $[-1,1]$, assert $\det M_{ij}>0$ and $|\det M_{ij}-1|$ is small, and treat $|\operatorname{tr}(R_{ij})|\approx 2$ as a degeneracy. With generic non‑Lagrangian ridges and canonical charts, $0<\rho_{ij}<1$ holds in practice.
- Cutoff (pruning). We accumulate $\rho$ along partial paths and prune when $\rho$ exceeds a configurable budget (default $\rho_{\max}=2$). This is a safe heuristic: increasing $\rho_{\max}$ never removes minimizers, while too‑small values may over‑prune; we choose a conservative default and log edges near degeneracy.

## Complexity and Practical Pruning
- Number of ridges and edges is polynomial in the input size, but cycle enumeration is exponential in worst case; pruning is essential.
- Fast rejections:
  - Precompute emptiness table for two-step patterns $(i\to j\to k)$ by checking whether $\psi_{ij}(\operatorname{dom}\psi_{ij})$ lies entirely outside $\operatorname{dom}\psi_{jk}$ (LP feasibility).  
  - Cache affine maps and half-space transforms to avoid recomputation.
  - Early action lower bounds from per-edge minima give a Dijkstra-like ordering over partial paths.
  - No facet revisits for minimizers: by Haim–Kislev’s “simple loop” theorem, there exists a minimizer that visits the interior of each facet at most once. We therefore restrict to cycles that do not repeat a facet (and hence not a 2-face), which sharply reduces the search. <!-- Reference: Haim-Kislev (2017), EHZ-polytopes.tex, Theorem simple_loop_theorem; see docs/src/thesis/bibliography.md. -->

## Tie-breaking (deterministic and performant)
When exit times to multiple co-facets are equal within tolerance, we need a deterministic choice that does not affect results but impacts performance.
- Options:
  - Lexicographic: choose the co-facet with smallest global index among the minimizers. Deterministic, O(1) overhead after scanning candidates.
  - Numeric ε‑slack: add a tiny $\varepsilon$ to denominators or RHS to break ties consistently (scale by facet norms to be dimensionless).
  - Seeded randomized: break ties using a seeded RNG per facet, fixed across runs for reproducibility.
- Implementation choice: Lexicographic with a symmetric tolerance window $|\tau_{ij}-\tau_{ik}|\le \varepsilon_\tau$. We set $\varepsilon_\tau = \varepsilon_{\mathrm{rel}}\cdot \max(1, \min(\tau_{ij},\tau_{ik})) + \varepsilon_{\mathrm{abs}}$ with small defaults (documented in code). <!-- Comment: deterministic, cheap, reproducible. -->

## Implementation Plan (Rust, with PyO3 bindings later)
- Geometry kernels (nalgebra):
  - Types for affine maps on $\mathbb{R}^2$ (`Mat2`, `Vec2`, offset), half-space representations in 2D, and simple 2D LP feasibility (or call out to a tiny solver).
  - Builders for domains $\operatorname{dom}\psi_{ij}$ from facet normals $(n_f,b_f)$ via the exit-time inequalities.
- Graphs:
  - Build 2-face digraph with per-edge data: $\operatorname{dom}\psi_{ij}$, $\psi_{ij}$, $\operatorname{im}\psi_{ij}$, $A_{ij}$, $\rho_{ij}$ (constant).
  - Optional: boolean table for $(i,j,k)$ emptiness.
- Search:
  - DFS with incumbent bound, candidate-set push-forward, action/rotation pruning, and fixed-point solve on closure.
  - Deterministic ordering for reproducibility; debug counters for pruned branches, visited edges, cycle lengths, etc.
- Output:
  - Best cycle, fixed point $z_\star$, action $A_\star$; lifted 4D polygonal curve via stored charts; provenance sidecar.

## Type Coverage and Assumptions
- We target Type 1 combinatorial orbits (segments inside facets; crossings at ridges) under the symplectic‑polytope assumption (no Lagrangian 2-faces). This aligns with the CH framework and the “simple loop” theorem in Haim–Kislev ensuring a minimizer visits each facet at most once.  
<!-- Docs: thesis/bibliography.md entries “Chaidez–Hutchings 2020/21” and “Haim‑Kislev 2017”. -->
<!-- review: if we must handle Lagrangian 2-faces in the future, we will extend the search to Type 2 trajectories and disable the rotation bound locally. -->

## Pseudocode (Rust‑ish)
```
struct RidgeId(u32);
struct FacetId(u32);

struct Aff2 { m: Mat2, t: Vec2 }  // z ↦ m*z + t
struct Aff1 { a: Vec2, b: f64 }   // z ↦ a·z + b

struct EdgeData {
    from: RidgeId,
    to: RidgeId,
    facet: FacetId,
    psi: Aff2,         // ψ_ij
    A_inc: Aff1,       // A_ij on domain
    rho_inc: f64,      // ρ_ij
    dom_i: Poly2,      // dom ψ_ij ⊂ A_i
    img_j: Poly2,      // im ψ_ij ⊂ A_j
}

struct State {
    ridges: Vec<RidgeId>,  // path
    facets_seen: BitSet,    // for no-revisit pruning
    C: Poly2,               // candidate polygon in A_{last}
    A: Aff1,                // accumulated action on A_{last}
    rho: f64,               // accumulated rotation
    Psi: Aff2,              // composed map to the start chart
}

fn extend(state: &State, e: &EdgeData, A_best: f64) -> Option<State> {
    if state.facets_seen.contains(e.facet) { return None; }
    let C_dom = intersect_poly(&state.C, &e.dom_i)?;
    let C1 = aff_image(&e.psi, &C_dom);
    let rho1 = state.rho + e.rho_inc;
    if rho1 > 2.0 { return None; }
    let A1 = compose_aff1(&state.A, &e.psi.inv()) + compose_aff1(&e.A_inc, &e.psi.inv());
    let C2 = intersect_halfspace(&C1, A1, A_best)?;  // { z : A1(z) ≤ A_best }
    Some(State {
        ridges: push(state.ridges, e.to),
        facets_seen: add(state.facets_seen, e.facet),
        C: C2, A: A1, rho: rho1,
        Psi: compose_aff2(&state.Psi, &e.psi),
    })
}
```

## Open Questions and Decisions Needed
1) Rotation normalization: pick Option A or B above and define the exact units so that index $3$ corresponds to $\rho\in(1,2)$.  
   <!-- ACTION: please confirm the preferred definition. -->
2) Simple cycles only: Haim–Kislev (2017) proves a “simple loop” theorem implying one can choose a minimizer that visits the interior of each facet at most once; we will enforce “no facet repeats” in the search.  
   <!-- ACTION: confirm this scope matches our targeted class (symplectic polytopes; non-Lagrangian 2-faces). We will note and handle the Lagrangian-face caveat separately if needed. -->
3) Handling ties/degeneracy: do we introduce a small symbolic perturbation (lexicographic) or numeric $\varepsilon$-slack?  
4) Initial bound $A_{\mathrm{best}}$: use bounding ball vs. a tighter volumetric bound; cite constants.  
5) Numeric robustness: adopt an “epsilon-violation warning mode” that allows a tiny slack around $C_p$ to survive rounding, as suggested.

## Experiments To Validate Design
- Sanity cases:
  - Polydisks and ellipsoids approximated by tight polytopes; check that results converge to the known $c_{\mathrm{EHZ}}$.  
  - Boxes and cross-polytopes in canonical positions; compare against literature/known inequalities for capacities and systolic ratio.
- Ablations:
  - With/without $(i,j,k)$ precomputation; effect on pruning rates.  
  - Pull-back vs. push-forward candidate updates; wall time and numerical stability.
- Scaling:
  - Random convex 4D polytopes with controlled facet counts; report cycles visited, pruned branches, and time-to-incumbent.

## Notes on Previous Draft
<!-- Comment: We have replaced the earlier mixed pull-back description with a single push-forward formulation (mutable in coordinates of the current ridge). This reduces repeated inverse-map applications and matches the “read ρ from ψ_ij” observation. -->
<!-- Comment: We made action increment explicit via A_ij(x) = (b_F/2)*τ_ij(x). This is affine on regions of constant first-exit, giving simple LPs for minima and feasibility. -->
<!-- Comment: Rotation is left as a one-number-per-edge choice; once fixed, we can implement it as a pure function of local facet/ridge frames. -->

## Code Links
- Rust workspace entry: `Cargo.toml`
- Native library (algorithms): `crates/viterbo`
- Python bindings (optional): `crates/viterbo-py`
- Orchestrator/pipelines: `src/viterbo/`
- Reproduction script: `scripts/reproduce.sh`

## Reviewer Checklist (delete after use)
- Assumptions match our intended class (non-Lagrangian 2-faces)?  
- Rotation: Option B normalization and units OK?  
- Numerical tolerances (ε_det, ε_feas, ε_τ) defaults acceptable?  
- Default A_best strategy OK until volume-based constant is cited?  
- “Simple loop” pruning enabled by default (per HK 2017)?  
- Chart orientation convention acceptable for cross-ridge rotation sign?  

## Clarifications (unstable, unsorted)
<!-- Purpose: park quick notes about code/spec divergences or open questions so agents can proceed without blocking on full edits. Treat entries as provisional; once stabilized, fold them into the main text and remove from this list. -->
- 1-faces not needed: under the stated genericity assumptions, minimizing cycles do not traverse 1-faces; the algorithm uses flow on facets and crossings at ridges only. The helper `geom4::reeb_on_edges_stub()` remains intentionally unimplemented.
- Orientation convention: we adopt the unique “natural” convention induced by the ambient symplectic form (require the chart orientation to agree with ω₀|_{face}). The implementation enforces this choice; no runtime toggle exists.

[^PreserveAlpha]: Standard fact in contact dynamics: for a contact form α with Reeb vector field R_α, the flow φ_t satisfies φ_t^*α=α and φ_t^*dα=dα since 𝓛_{R_α}α=i_{R_α}dα+d(α(R_α))=0.
[^ReturnArea]: Poincaré first‑return maps of Reeb flows on 3‑dimensional contact manifolds are area‑preserving with respect to dα on any transverse surface of section; see e.g. Albers–Geiges–Zehmisch (2018).
[^CH]: Chaidez–Hutchings (2021): “Computing Reeb dynamics on four‑dimensional convex polytopes”, J. Comput. Dyn. 8(4):403–445; arXiv:2008.10111.
