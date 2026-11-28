- B. **Optimal Control Design for Hyperbolic PDE model**

The first step to derive the state feedback control policy is to write the PDE in its equivalent state space formulation on a Hilbert space H [23]. For a linear hyperbolic PDE of the form

$$
\frac{\partial x}{\partial t}=V\frac{\partial x}{\partial z}+Mx+B_{0}u
$$

(15)

$$
y=C_{0}x
$$

(16) this results in x˙(t) = Ax(t) + Bu(t) (17)

$$
y(t)=Cx(t)
$$

(18) where A is the linear operator defined as

$$
A=V\cdot\frac{d.}{dz}+M\cdot I
$$

(19) on the domain dx D(A) = {x ∈ H : ∈ H and x(0) = 0} (20) dz with $V\in\mathbb{R}^{n\times n}$ being a symmetric matrix and $M,B_{0}$, and $C_{0}$ being real continuous space-varying matrix functions. In addition, x is assumed to be absolutely continuous. It is proven in [23] that if V \< 0 then A generates an exponentially stable $C_{0}$-semigroup. The linear-quadratic optimal control problem on an infinite time interval finds a control input $u_{opt}$ such that the functional $J(x_{0},u)$ is minimized. The cost functional is given as Z

∞

J(x~0~, u) = hCx(t), QCx(t)i + hu(t), Ru(t)i dt (21)

0

The solution of (21) can be found by finding the non-negative self-adjoint operator P which solves

$$&#91;A^{&#42;}P+PA+C^{&#42;}QC-PBR^{-1}B^{&#42;}P&#93;x=0$$

(22) When (A, B) is exponentially stabilizable and $(Q^{1_{2}}C,A)$ is exponentially detectable (22) has a unique, non-negative solution P and (21) is minimized by the unique control input $u_{opt}$ given by

$$
u_{opt}(t)=K_{0}x(t)
$$

(23) where the optimal feedback is

$$
K_{0}=-R^{-1}B^{*}P
$$

(24) The proof that (A, B) is exponentially stabilizable and $(P^{1_{2}}C,A)$ is exponentially detectable given V \< 0 is shown in [23]. Furthermore,

$$
P:=\Phi(z)I
$$

(25) where Φ(z) is the solution of the matrix Riccati differential equation given by

$$
V\frac{d\Phi}{dz}=M^{*}\Phi+\Phi M+C_{0}^{*}Q_{0}C_{0}-\Phi B_{0}R_{0}^{-1}B_{0}^{*}\Phi
$$

(26)

$$
\Phi(L)=0
$$

where $Q_{0}$ is a positive matrix and $R_{0}$ is a self-adjoint positive matrix. Thus, (24) becomes

$$
K_{0}=-R_{0}^{-1}(z)B_{0}^{*}(z)\Phi(z)I
$$

(27)

- C. **LQ Control Deisgn for LWR Model with VSL**

In order to find Φ(z), we take (14) and transform it into ∂∆ρ ρ~0~ ∂∆ρ = −b~0~U~max~ 1 − 2 − ∂t ρ~max~ ∂z ∂∆b

$$
\rho_{0}U_{max}1-\frac{\rho_{0}}{\rho_{max}}
$$

(28) ∂z so that it is now in the form of (15). It follows that

$$
V=-b_{0}U_{max}1-2\frac{\rho_{0}}{\rho_{max}}
$$

$$
M=0
$$

(29)

$$
B_{0}=-\rho_{0}U_{max}1-\frac{\rho_{0}}{\rho_{max}}
$$

$$
C_{0}=I
$$

The condition of V \< 0 is met if $\rho_{0}<\frac{1}{2}\rho_{max}$ in (29). This means that $\rho_{0}$ must always fall in the free flow regime, which is a valid assumption since the equilibrium profile should never fall in the congested regime to avoid traffic jams. The control input for the LWR model with VSL is the change of the speed limit over the stretch of highway. Using (26) and (29) the matrix Riccati equation becomes

$$
V\frac{d\Phi}{dz}=Q_{0}-\Phi B_{0}R_{0}^{-1}B_{0}^{*}\Phi
$$

(30)

$$
\Phi(L)=0
$$

As in [23], $R_{0}=1$. The values for V and $B_{0}$ will be inserted into the equation at the end of the derivation for Φ for simplicity. Using separation of variables, (30) becomes

$$
Z_{\Phi}
$$

Z

z

V dΦ = dz = z + C (31) Q~0~ − B~0~ <sup>2</sup>Φ<sup>2</sup>

0 0

The left hand side of (31) can be solved by a combination of integration by partial fractions and substitution. After applying both methods, (31) becomes V p p √ (ln $|B_{0}Φ + Q_{0}| − ln |B_{0}Φ − Q_{0}|) = z + C$

2B~0~ Q~0~ (32) Then, √ √ B~0~Φ + Q~0~

$$
2B_{0} Q_{0} ln √ = (z + C) B_{0}Φ − Q_{0} V √ √ B_{0}Φ + Q_{0}
$$

$$
2B_{0} Q_{0} √ = exp (z + C) B_{0}Φ − Q_{0} V √ p p
$$

$2B_{0} Q_{0} B_{0}Φ+ Q_{0} = B_{0}Φ− Q_{0} exp (z+C) V (33)$ The solution to (33) can be either √ p p

2B~0~ Q~0~ B~0~Φ + Q~0~ = (B~0~Φ − Q~0~) exp (z + C) V (34) or √ p p

$$
2B_{0} Q_{0} B_{0}Φ + Q_{0} = −(B_{0}Φ − Q_{0}) exp (z + C) V (35)
$$
