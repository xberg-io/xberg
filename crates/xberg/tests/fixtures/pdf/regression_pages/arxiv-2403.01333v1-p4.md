the matrix inequality in (6) can be written as 

$$
YB_{d}YB_{u}
$$

$$
C_{z}^{T}
$$

$$
P+P^{T}
$$

**0 0 0 0**

$$
0\leq0,
$$

 ∗

$$-\gamma\frac{W_{d}^{-2}}{0}$$



$$
\mathrm{diag}(\kappa_{a})
$$

**0**

∗ ∗

$$-\gamma I$$

which is convex in $Y,V,\omega_{c}$ , and $\kappa_{a}$ , where

**Y A**

$$
YB_{u}
$$

**P**

:= .

**V**

−**diag** (**ω**~c~)

**where** $\lambda_{a}\geq0,\lambda_{\omega_{c}}\geq0$**, and** $\lambda_{x_{F}}\geq0$ are user-defined weights, and $W_{d}$ is a given scaling matrix for d(t). The controller gain can be recovered as $K:=\mathrm{diag}(\omega_{c})^{-1}V.$ Proof.

The condition for bounding the $\mathcal{H}&#95;{2}$ norm

**d¯**

from

$$
\rightarrow
$$

**z**

is given by

$XM_{11}+$ $w_{a}$¯

2

**W**

0

T d T

**M X** + **XM**~12~ **M X**

$$
\leq
$$

0, and

11 2 12

0 **W**~a~ h i

$$
\mathrm{tr}M_{21}X^{-1}M_{21}^{T}
$$

≤ γ [15], which for the problem considered are the following LMIs   The DC gain matrix of the actuator is minimized, by

$$
YB_{d}YB_{u}
$$

h i

$$
P+P^{T}
$$

**0 0**

minimizing a upper bound of $\mathrm{tr}V^{T}V$ , i.e., minimize $\gamma_{x_{F}}$

$$
\leq0,
$$

−2



**W 0** ~d~

such that $V^{T}V\leq Q$ and $\mathrm{tr}&#91;Q&#93;\leq\gamma_{x_{F}}$ and **Q** ≥ 0. The ∗ −

**0**

$$
\mathrm{diag}(\kappa_{a})
$$

constraint on **V** can be expressed as the following LMI using Schur complement,

$$
QC0
$$

T

$$&#42;^{1}Y^{z}0\geq0,\mathrm{tr}&#91;Q_{1}&#93;\leq\gamma.$$

**Q V**

≥ 0.

∗ ∗

**I**

## V I

h i The actuator noise is maximized by minimizing $k\kappa_{a}k_{2}$, the Similar to theorem 1, $\mathrm{tr}V^{T}V$ $\leq\gamma_{x_{F}}$ is equivalent control rate is minimized by minimizing $k\omega_{c}k_{{}^{2}x_{F}}$, and the con-

$Q_{2}V^{T}$

to ≥ 0 and $\mathrm{tr}&#91;Q_{2}&#93;\leq\gamma_{xF}$ . Maximum actrol magnitude is minimized by minimizing γ . Therefore, the cost function for the optimization is

$$
{}_{\kappa_{a},\omega_{c},\gamma_{x}}\min{}_{\geq0,Q\geq0}\lambda_{a}k\kappa_{a}k_{2}+\lambda_{\omega_{c}}k\omega_{c}k_{2}+\lambda_{x_{F}}\gamma_{x_{F}},
$$

$$
{}_{F},V,Y
$$

where $\lambda_{a},\lambda_{\omega_{c}}$, and $\lambda_{x_{F}}$ are user-defined weights.

## V I

tuator degradation is achieved by minimizing λ kκ k~2 a a~ + $\lambda_{\omega_{c}}k\omega_{c}k_{2}+\lambda_{x_{F}}\gamma_{x_{F}}$ , where $\lambda_{a},\lambda_{\omega_{c}}$, and λ are user~**x**~~F~ defined weights.

Remark 3. In both theorem 1 and 2, the minimum control rate is given by the elements $\omega_{c}$, the minimum control magnitude

We next present the result for $\mathcal{H}&#95;{2}$ optimal closed-loop

**is characterized by** $\gamma_{x_{F}}$ **, and the maximum actuator noise is**

√ performance. **characaterized by the scaling** diag 1/ **κ , where the square-**

Theorem 2. For the system in (3) with $D_{d}=0$, the state- root and the inverse are defined to be element-wise.

Therefore, the optimal $\omega_{c},\gamma_{x_{F}}$ , and κ quantities maximum feedback gain that guarantees

$\frac{d}{w_{a}}\rightarrow z{}&#95;{2}\leq\gamma$ **for a**

**actuator degradation (as defined above) for which a state-**

**given** $γ ≥ 0$ **, with maximum actuator noise, and minimum**

**feedback controller is able to achieve the user specifiedcontrol magnitude and rate, is given by the following convex optimization problem,**

$$
{}_{\kappa_{a},\omega_{c},\gamma_{x}}\min_{Y,Q_{1},Q_{2}}\lambda_{a}k\kappa_{a}k_{2}+\lambda_{\omega_{c}}k\omega_{c}k_{p}+\lambda_{x_{F}}\gamma_{x_{F}},
$$

$$
{}_{F},V,
$$

**subject to**

 

$$
YB_{d}YB_{u}
$$

$$
P+P^{T}
$$

**0 0**

$$
\leq0
$$

−2



**W 0** ~d~

∗ −

**0**

$$
\mathrm{diag}(\kappa_{a})
$$

$$
QC0
$$

z

1 T

**Q**~2~ **V**

 ∗ 

**Y**

$$
0 ≥ 0, ≥ 0,
$$

## V I

∗ ∗

**I**

$$
\mathrm{tr}[Q_{1}]\leq\gamma,\mathrm{tr}[Q_{2}]\leq\gamma_{x_{F}},\kappa\in\mathbb{R}_{+}^{nu},\omega_{c}\in\mathbb{R}_{+}^{nu},\gamma_{x_{F}}\in\mathbb{R}_{+},
$$

$$
V\in\mathbb{R}^{nu\times nx},Y\in\mathbb{S}_{+}^{nx},Q_{1}\in\mathbb{S}_{+}^{nz},Q_{2}\in\mathbb{S}_{+}^{nx},
$$

**with Y A**

$$
YB_{u}
$$

**P**

:= ,

**V**

−diag(**ω**~c~)

**performance in terms of the closed-loop systems** $\mathcal{H}&#95;{2}$ **or** $\mathcal{H}&#95;{\infty}$

**norms.**

IV. EXAMPLES

- A. **F-16 Flight Control Application**

We apply the results to the design of a full-state feedback controller for a model of the longitudinal motion of the F-16 aircraft. The model is based on the aerodynamics from the NASA Technical Report 1538 [16]. The nonlinear model is trimmed at 10000 ft at 900 ft/s for steady-level flight. The longitudinal states are θ (pitch angle), $V_{t}$ (total velocity), α

- 
¯ (angle-of-attack), q (pitch rate); with trim value $θ = 5.95$ , ¯

V${}&#95;{t}=900ft/s,\alpha=5.95^{\circ},q$¯ ¯ = 7.85 deg/s. The control variables are T (thrust), $\delta_{e}$ (elevator angle), and $\delta_{lef}$ (leading ¯ ¯ flap); with trim values T $=10461.84lb,\delta_{e}=-3.82^{\circ}$, and $\delta_{lef}=12.42^{\circ}$. We assume the aircraft experiences disturbance in the angle-of-attack α, with $W_{d}=0.01.$ The open-loop system in (2) is given by the following system matrices
