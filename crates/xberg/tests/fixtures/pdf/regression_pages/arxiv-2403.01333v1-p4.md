**the matrix inequality in (6) can be written as where λ**~**a**~ **≥ 0, λ**~**ω**~ **≥ 0, and λ**~**x**~ **≥ 0 are user-defined c F**

 

**weights, and W**~**d**~ **is a given scaling matrix for d(t). The**

**1**

**Y B**~**d**~ **Y B**~**u**~ **C**~**z**~ <sup>**T**</sup> **controller gain can be recovered as K :=** diag **(ω )**<sup>**−**</sup>

**V .**

**P + P**<sup>**T**</sup>

**c**

 0 0  0  

**2**

 

**Proof. The condition for bounding the H norm W**<sup>**−**</sup>

0 0

$$
≤ 0,
$$

**2**



**d**



$$
∗ −γ
$$

¯

**d**

  0

**diag (κ )**

0

**a**

**from → z is given by XM**~**11**~ **+ w**~**a**~

¯

**∗ ∗ −γI 2**

**W**~**d**~ **0 M**<sup>**T**</sup> **X + XM M**<sup>**T**</sup> $12 X ≤ 0$, and 11 2 12 which is convex in Y , V , ω~**c**~**, and κ**~**a**~**, where**

**0 W**~**a**~

h i

**1**

**tr M X**<sup>**−**</sup> **M**<sup>**T**</sup> **≤ γ [15], which for the problem Y A Y B**~**u**~

**21 21 P := . V −diag(ω ) considered are the following LMIs c**

 

**The DC gain matrix of the actuator is minimized, by Y B**~**d**~ **Y B**~**u**~

h i

**P + P**<sup>**T**</sup>

**T**

 0 0 

**minimizing a upper bound of tr V V , i.e., minimize γ**~**xF**~

 

$$
≤ 0,
$$

**2**

 

**T**

**W**<sup>**−**</sup>

0

**d**

**such that $V V ≤ Q$ and tr [Q] ≤ γ**~**x**~ **and $Q ≥ 0$. The F ∗ −**

0

**diag (κ**~**a**~**) constraint on V can be expressed as the following LMI using**

 

**Schur complement, Q**~**1**~ **C**~**z**~

0  

**T**

$$
∗ Y 0 ≥ 0, tr [Q1] ≤ γ.
$$

$Q V ≥ 0$.

**∗ ∗ I**

### V I

h i

**T**

**The actuator noise is maximized by minimizing kκ k , the Similar to theorem 1, tr $V V ≤ γ$ is equivalent**

**a 2 x**~**F**~

**control rate is minimized by minimizing kω k , and the con- T c 2**

**Q**~**2**~ **V to ≥ 0 and tr $&#91;Q &#93; ≤ γ$ . Maximum actrol magnitude is minimized by minimizing γ . Therefore,**

**2 xF x**~**F**~

**V I the cost function for the optimization is**

**tuator degradation is achieved by minimizing λ**~**a**~**kκ k**~**a**~ **+** ~**2**~ **λ**~**ωc**~ **kω k**~**c**~ **+ λ**~**xF**~**γ**~**xF**~**, where λ**~**a**~**, λ**~**ωc**~**, and λ**~**xF**~ **are user2 min λ**~**a**~**kκ k**~**a**~ **+ λ**~**ω**~ **kω k**~**c**~ **+ λ**~**x**~ **γ , 2 c 2 F x**~**F**~

$$
κ ,ω ,γ ,V ,Y ≥0,Q≥0
$$

**defined weights.**

**a c xF**

**where λ**~**a**~**, λ**~**ω**~ **, and λ**~**x**~ **are user-defined weights. c F**

**Remark 3. In both theorem 1 and 2, the minimum control rate is given by the elements ω**~**c**~**, the minimum control magnitude**

**We next present the result for H**~**2**~ **optimal closed-loop is characterized by γ**~**x**~ **, and the maximum actuator noise is F √ performance. characaterized by the scaling** diag **1/ κ , where the square-**

**Theorem 2. For the system in (3) with $D = 0$, the state- root and the inverse are defined to be element-wise.**

**d**

**d Therefore, the optimal ω**~**c**~**, γ**~**x**~ **, and κ quantities maximum F feedback gain that guarantees → $z ≤ γ$ for a w**~**a**~ **actuator degradation (as defined above) for which a state-**

**2**

**given $γ ≥ 0$, with maximum actuator noise, and minimum feedback controller is able to achieve the user specifiedcontrol magnitude and rate, is given by the following convex performance in terms of the closed-loop systems H**~**2**~ **or H**~**∞**~ **optimization problem, norms.**

- IV. **EXAMPLES**

$$
min λakκ ka + λω kω kc + λ γ , 2 c
$$

$$
κ ,ω ,γ ,V ,Y ,Q1,Q a c xF 2
$$

**subject to**

 

**Y B**~**d**~ **Y B**~**u**~ **P + P**<sup>**T**</sup>

 0 0   

$$
≤ 0
$$

**2**

 

**W**<sup>**−**</sup>

0

**d ∗ −**

0 diag **(κ**~**a**~**)**  

**Q**~**1**~ **C**~**z**~

0

**T**

**Q**~**2**~ **V**

 

**∗ $Y 0 ≥ 0, ≥ 0$,**

### V I

### ∗ ∗ I

**p x**~**F**~ **x**~**F**~

- A. **F-16 Flight Control Application We apply the results to the design of a full-state feedback controller for a model of the longitudinal motion of the F-16 aircraft. The model is based on the aerodynamics from the NASA Technical Report 1538 [16]. The nonlinear model is trimmed at 10000 ft at 900 ft/s for steady-level flight. The longitudinal states are θ (pitch angle), V**~**t**~ **(total velocity), α**

- 
**¯ (angle-of-attack), q (pitch rate); with trim value $θ = 5.95$ ,**

- 
**¯V**~**t**~ **= 900 ft/s, α¯ = 5.95 , q¯ = 7.85 deg/s. The control variables are T (thrust), δ**~**e**~ **(elevator angle), and δ**~**lef**~ **(leading** tr $&#91;Q &#93; ≤ γ$, tr $&#91;Q &#93; ≤ γ$ , κ ∈ R<sup>**nu**</sup>**, ω ∈** R<sup>**nu**</sup>**, \$γ ∈** R~**+**~\$**,**

**1 2 x**~**F**~ **+ c + x**~**F**~

- 
**¯ ¯ flap); with trim values $T = 10461.84$ lb, δ**~**e**~ **= −3.82 , and**

$$
×nx
$$

$$
V ∈ Rnu , Y ∈ Snx, Q ∈ Snz, Q ∈ S+ nx,
$$

- 
$$+ 1 + 2$$

**δ**~**lef**~ **= 12.42 . We assume the aircraft experiences disturbance in the angle-of-attack α, with W**~**d**~ **= 0.01. with The open-loop system in (2) is given by the following Y A Y B**~**u**~ **P := , system matrices V −** diag **(ω**~**c**~**)**
