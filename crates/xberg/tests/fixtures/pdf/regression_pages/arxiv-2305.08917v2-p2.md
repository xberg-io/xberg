attractive choice for simulation when applicable. Correspondingly, exciting opportunities open up when new interesting physical models are found which can be simulated using this method [46–48]. Recently, there has been intense experimental development to study the physics of confinement and quantum spin liquids [49–55] using tools of quantum simulation and computation. The microscopic models used to capture the physics involve fermions interacting with (Abelian) gauge fields. In this Letter, we develop meron cluster algorithms for a class of experimentally relevant models [56, 57], enabling a robust elucidation of their phase diagrams. We also introduce new classes of $\mathbb{Z}&#95;{2}$ and U(1) multi-flavored gauge-fermion theories, which might be realized in cold-atom setups and also be further studied using Monte Carlo techniques. Notably, the U(1) family of these models seems to be one of the few families that falls outside the class of models known to be simulable by auxiliary-field methods, as are [23, 58, 59]. Moreover, the worldline nature of the method makes it easily employed to study the corresponding phases in these theories in higher spatial dimensions, and the resulting physically-relevant configurations are promising inputs for machine learning algorithms. Models.– We start with the half-filled t-V model–a spinless fermionic Hamiltonian involving only the most local interactions,

$$
H=\frac{X}{\langle xy\rangle}-\frac{t}{2}(c_{x}^{\dagger}c_{y}+c_{y}^{\dagger}c_{x})+V(n_{x}-\frac{1}{2})(n_{y}-\frac{1}{2})
$$

(1) Here ⟨xy⟩ are the nearest neighbor site pairs, $c^{\dagger},c$ are the creation and annihilation operators respectively, and the repulsive interaction V is given in terms of the number operator $n=c^{\dagger}c$. It is simulable by meron clusters for V ≥ 2t [45, 48]. In this Letter, we extend the meron cluster method to physically interesting Hamiltonians involving gauge fields, which are lower-dimensional versions of quantum electrodynamics (QED) [60, 61]. The $\mathbb{Z}&#95;{2}$- and U(1)-gauge symmetric families are given by:

des

$H_{N_{f}}^{(g)}=-\frac{XY^{N_{f}}}{\langle xy\rangle f=1}H_{\langle xy\rangle,f}^{(g)}+H_{\langle xy\rangle,f}^{(g),}$ (2) The label $g\in\\{U(1),\mathbb{Z}&#95;{2}\\}$ is the gauge symmetry, with

$$
H_{\langle xy\rangle,f}^{\mathbb{Z}_{2}}=tc_{x,f}^{\dagger}s_{xy,f}^{1}c_{y,f}+c_{y,f}^{\dagger}s_{xy,f}^{1}c_{x,f}
$$

(3)

$$
H_{\langle xy\rangle,f}^{U(1)}=tc_{x,f}^{\dagger}s_{xy,f}^{+}c_{y,f}+c_{y,f}^{\dagger}s_{xy,f}^{-}c_{x,f}.
$$

The hopping of spinless fermions between the nearest neighbors ⟨xy⟩ are now governed by the presence of gauge fields, represented by spin-1/2 operators, $s_{xy}^{k}$, on the bond. Fig. 1 illustrates the model degrees of freedom.

des

Then $H_{\langle xy\rangle,f}^{(g),}$ is a designer term [62] that makes the mod-

2

fermion hole spin-1/2 up spin-1/2 down FIG. 1: Example fermion occupations and bond variables for the theories on a square spatial lattice with $N_{f}=1.$ els simulable by the meron algorithm, des$H_{\langle xy\rangle,f}^{\mathbb{Z}&#95;{2},}=-2tn_{x,f}-\frac{1}{2}$

$$
n_{y,f}-\frac{1}{2}+\frac{t}{2}
$$

des (4)$H_{\langle xy\rangle,f}^{U(1),}$ $=-tn_{x,f}-\frac{1}{2}$

$$
n_{y,f}-\frac{1}{2}
$$

$$-ts_{xy,f}^{3}(n_{y,f}-n_{x,f})+\frac{t}{4}$$

These terms serve a similar role as the V = 2t term in the meron cluster algorithm applied to the t-V model [45], and similarly an additional particle-hole symmetric V ≥ 0 term can be added to the models here in a signproblem-free way. In the $\mathbb{Z}&#95;{2}$ gauge theory, the gauge field $s^{1}=\sigma^{1}/2$ couples to fermions, and the local $\mathbb{Z}&#95;{2}$ symmetry is manifest via the commutation${}&#95;{{}&#95;{\alpha,f}s}{}&#95;{3}{}&#95;{x}&#91;Q_{x},H^{\mathbb{Z}&#95;{2}}&#93;=0,$ where `Q_{x}=(-1)^{P}{}^{{}_{{}_{f}n_{x,f}}Q}`${}&#95;{f,\alpha}s_{x,x}^{3}$ ${}&#95;{-\alpha,x,f},\alpha$ˆ are the ~ˆ~ +ˆ ˆ unit vectors in a d-dimensional square lattice. For the U(1) theory, the unitary operator (1), which commutes with is given by $V_{-^{U}}{}&#95;{(1)}{}&#95;{s_{x}^{3}}=^{Q^{V_{U}}}{}&#95;{x}e^{i\theta xGx}G_{x}=$, with i$P_{f}\frac{{}&#95;{h}H}{n_{x,f}}\frac{U(1)}{-}P$ ${}&#95;{\alpha}s_{x,x}^{3}$ ${}&#95;{-\alpha,x,f}+((-1)^{x}-1)/2.$~ˆ +ˆ~α,f ~ˆ~ In the terminology of gauge fields, our microscopic models are quantum link models [63], which realize the continuous gauge invariance using finite-dimensional quantum degrees of freedom. The identification with usual gauge field operators is $U_{xy,f}=s_{xy,f}^{+},U_{xy,f}^{\dagger}=s_{xy,f}^{-},E=s_{xy,f}^{3}.$ We note that a straightforward application of the meron idea necessitates the introduction of an equivalent flavor index for gauge links as fermion flavors. Naively, the total Gauss law can be expressed through a product $(\mathbb{Z}&#95;{2})$ or sum (U(1)) of the Gauss law of individual flavors degrees of freedom, and the resulting theories have $\mathbb{Z}&#95;{2}^{\otimes N_{f}}$ and $U(1)^{\otimes N_{f}}$ gauge symmetry. However, flavored gauge-interactions can also be turned on in the U(1) model (as explained in the Supplementary Material),

$$
H_{N_{f}=2}^{U(1)}\rightarrow H_{N_{f}=2}^{U(1)}+J\frac{X}{\langle xy\rangle}s_{xy,1}^{3}s_{xy,2}^{3},
$$

(5) or through a Hubbard-U interaction for both $\mathbb{Z}&#95;{2}$ and U(1)-symmetric models [48]. These additions directly
