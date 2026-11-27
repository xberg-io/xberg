And considering (3.22), we have the following estimate s ¯$u_{i,h}-E_{i,h}^{(\ell+1)}u_{i,h}$¯ ¯ $\leq\lambda_{i,h}1+&#95;{\lambda_{1}}\frac{\eta_{a}^{2}(W_{H})}{\delta_{\lambda}^{(\ell+1)}}&#95;{2}1+\frac{1}{\lambda_{1}\delta_{\lambda}^{(\ell)}}\eta_{a}^{2}(W_{H})u_{i,h}-E_{i,h}^{(\ell)}u_{i,h}$

a,h

The following $\parallel\cdot\parallel_{b}$-error estimate holds \!

$u_{i,h}-E_{i,h}^{(\ell+1)}u_{i,h}$¯ ¯ ${}&#95;{b}\leq1+\frac{1}{\lambda_{1}\delta_{\lambda}^{(\ell+1)}}\eta_{a}(W_{H})u_{i,h}-E_{i,h}^{(\ell+1)}u_{i,h}$

From (3.23) and (3.24), the proof is completed.

the following error estimates $u_{i,h}-E_{i,h}^{(\ell+1)}u_{i,h}$¯ ¯

$$
{}_{a,h}\leq
$$

$u_{i,h}-E_{i,h}^{(\ell+1)}u_{i,h}$¯ ¯

$$
{}_{b}\leq
$$

**Corollary 3.1.** Under the conditions of Theorem 3.2, the eigenfunction approximation $u_{i,h}^{(\ell+1)}$

¯$\gamma(\lambda_{i,h})^{\ell}$ $1+\frac{1}{\lambda_{1}\delta_{\lambda}^{(\ell+1)}}\eta_{a}(W_{H})u_{i,h}-E_{i,h}^{(\ell+1)}u_{i,h}$ $u_{i,h}-E_{i,h}^{(1)}u_{i,h}$¯ \!

\! ¯

¯

¯

$$
{}_{a,h}.
$$

(3.23)

¯

$$
{}_{a,h}.
$$

(3.24)

¯

$$
{}_{a,h},
$$

¯

has

(3.25) ¯

$$
{}_{a,h},
$$

(3.26)

where s \! ¯ ¯$\gamma(\lambda_{i,h})=\lambda_{i,h}1+&#95;{\lambda_{1}}\frac{\eta_{a}^{2}(W_{H})}{\delta_{\lambda}^{(\ell+1)}}&#95;{2}1+\frac{1}{\lambda_{1}\delta_{\lambda}^{(\ell)}}\eta_{a}^{2}(W_{H}).$ (3.27)

The error estimate for the eigenvalue approximation $\lambda_{i,h}^{(\ell)}$ can be deduced from Theorem 3.2 and

Remark 3.2.

# 4 Numerical experiments

In this section, numerical experiments are presented to validate our theoretical results. Here, we are concerned with the Laplace eigenvalue problem (2.1), where the computing domain is set to be the unit square Ω = (0$,1)\times(0,1).W_{H}$ and $V_{h}$ are chosen as the conforming linear element and the CR element spaces defined on the coarse mesh $\mathcal{T}&#95;{H}$ and the fine mesh $\mathcal{T}&#95;{h}$, respectively. The fine mesh $\mathcal{T}&#95;{h}$ is obtained from the coarse mesh $\mathcal{T}&#95;{H}$ by the regular refinement of uniform triangular mesh.

Since the coarse space $W_{H}$ is the conforming linear finite element space defined on the coarse mesh $\mathcal{T}&#95;{H}$, together with the theories of the error estimates of CR element and conforming linear element [8, 11], it is known that the following estimate holds when the mesh size h \< H, $\eta_{a}(W_{H})\leq_{f\in L^{2}}\sup_{f\parallel_{b}=1}\parallel Tf-T_{h}f\parallel_{a,h}+&#95;{v_{H}}\inf_{H}\parallel Tf-v_{H}\parallel_{a}$

$$
\in W
$$

- (Ω) ,∥

$$
\leq CH,
$$

where the constant C depends on shape of the meshes $\mathcal{T}&#95;{H}$ Based on Theorems 3.1 and 3.2, the convergence results can be concluded with the following inequalities $u_{i,h}-F_{k,h}^{(\ell+1)}u_{i,h}$¯ ¯ ${}&#95;{a,h}\leq CCH^{2\ell}$ $u_{i,h}-F_{k,h}^{(1)}u_{i,h}$¯

16

and $\mathcal{T}&#95;{h}.$ ¯

$$
{}_{a,h},
$$

$$
i=1,\cdot\cdot\cdot,k,
$$

(4.1)
