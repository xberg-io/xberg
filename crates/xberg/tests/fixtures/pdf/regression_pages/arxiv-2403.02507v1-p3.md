- B. **Optimal Control Design for Hyperbolic PDE model The first step to derive the state feedback control policy is to write the PDE in its equivalent state space formulation on a Hilbert space H [23]. For a linear hyperbolic PDE of the form ∂x ∂x = V + Mx + B**~**0**~ **u (15) ∂t ∂z y = C**~**0**~ **x (16)**

**this results in x ˙( t ) = Ax ( t ) + Bu ( t ) (17) y ( t ) = Cx ( t ) (18) where A is the linear operator defined as d. A = V**

- **+ M**
- **I (19) dz on the domain dx D ( A ) = { x ∈ H : ∈ H and x (0) = 0 } (20) dz**

**n×n**

**with V ∈** R **being a symmetric matrix and M, B**~**0**~**, and C**~**0**~ **being real continuous space-varying matrix functions. In addition, x is assumed to be absolutely continuous. It is proven in [23] that if V \< 0 then A generates an exponentially stable C**~**0**~**-semigroup. The linear-quadratic optimal control problem on an infinite time interval finds a control input u**~**opt**~ **such that the functional J(x ,**~**0**~ **u) is minimized. The cost functional is given as Z**

**∞**

**J(x ,**~**0**~ **u) = hCx(t), QCx(t)i + hu(t), Ru(t)i dt (21)**

**0 The solution of (21) can be found by finding the non-negative self-adjoint operator P which solves ∗ ∗ −1 ∗ [A P + PA + C QC − PBR B P]x = 0 (22)**

1

**When (A, B) is exponentially stabilizable and (Q**<sup>2</sup>**C, A) is exponentially detectable (22) has a unique, non-negative solution P and (21) is minimized by the unique control input u**~**opt**~ **given by u**~**opt**~**(t) = K**~**0**~ **x(t) (23) where the optimal feedback is**

**−1 ∗**

**K**~**0**~ **= −R B P (24) The proof that (A, B) is exponentially stabilizable and**

1

**(P**

2**C, A) is exponentially detectable given V \< 0 is shown in [23]. Furthermore, P := Φ(z)I (25) where Φ(z) is the solution of the matrix Riccati differential equation given by dΦ ∗ ∗ −1 ∗**

**V = M Φ + ΦM + C Q**~**0**~ **C**~**0**~ **− ΦB R**~**0**~ **B**~**0**~ **0 Φ 0 dz (26) Φ(L) = 0 where Q**~**0**~ **is a positive matrix and R**~**0**~ **is a self-adjoint positive matrix. Thus, (24) becomes**

**−1 ∗**

**K**~**0**~ **= −R (z)B (z)Φ(z)I (27) 0 0**

- C. **LQ Control Deisgn for LWR Model with VSL In order to find Φ(z), we take (14) and transform it into ∂∆ρ ρ**~**0**~ **∂∆ρ = −b** ~**0**~**U**~**max**~ **1 − 2 − ∂t ρ**~**max**~ **∂z ρ**~**0**~ **∂∆b ρ U**~**0 max**~ **1 − (28) ρ**~**max**~ **∂z so that it is now in the form of (15). It follows that**

**ρ**~**0**~

**V = −b**~**0**~ **U**~**max**~ **1 − 2 ρ**~**max**~ **M = 0 (29) ρ**~**0**~ **B**~**0**~ **= −ρ U**~**0 max**~ **1 − ρ**~**max**~ **C**~**0**~ **= I**

**1**

**The condition of V \< 0 is met if ρ**~**0**~ **\< ρ**~**max**~ **in (29). This 2 means that ρ**~**0**~ **must always fall in the free flow regime, which is a valid assumption since the equilibrium profile should never fall in the congested regime to avoid traffic jams. The control input for the LWR model with VSL is the change of the speed limit over the stretch of highway. Using (26) and**

- (29) **the matrix Riccati equation becomes**

**dΦ −1 ∗**

**V = Q**~**0**~ **− ΦB R**~**0 0**~ **B**~**0**~**Φ dz (30) Φ(L) = 0 As in [23], R**~**0**~ **= 1. The values for V and B**~**0**~ **will be inserted into the equation at the end of the derivation for Φ for simplicity. Using separation of variables, (30) becomes Z Z Φ**

**z**

**V dΦ = dz = z + C (31) Q**~**0**~ **− B**~**0**~ <sup>**2**</sup>**Φ**<sup>**2**</sup>

**0 0**

**The left hand side of (31) can be solved by a combination of integration by partial fractions and substitution. After applying both methods, (31) becomes V p p √ (ln |B**~**0**~**Φ + Q**~**0**~**| − ln |B**~**0**~**Φ − Q**~**0**~**|) = z + C**

**2B**~**0**~ **Q**~**0**~ **(32) Then, √ √ B**~**0**~**Φ + Q**~**0**~

**2B**~**0**~ **Q**~**0**~ **ln √ = (z + C) B**~**0**~**Φ − Q**~**0**~ **V √ √ B**~**0**~**Φ + Q**~**0**~

**2B**~**0**~ **Q**~**0**~ **√ = exp (z + C) B**~**0**~**Φ − Q**~**0**~ **V √ p p**

**2B**~**0**~ **Q**~**0**~ **B**~**0**~**Φ+ Q**~**0**~ **= B**~**0**~**Φ− Q**~**0**~ **exp (z+C) V (33) The solution to (33) can be either √ p p**

**2B**~**0**~ **Q**~**0**~ **B**~**0**~**Φ + Q**~**0**~ **= (B**~**0**~**Φ − Q**~**0**~**) exp (z + C) V (34) or √ p p**

**2B**~**0**~ **Q**~**0**~ **B**~**0**~**Φ + Q**~**0**~ **= −(B**~**0**~**Φ − Q**~**0**~**) exp (z + C) V (35)**
