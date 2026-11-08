|  |  | 10 5  |
| --- | --- | --- |
| 0 . 0 | 0 . 0 0 . 0 | 1 . 0 4 |
|   − 32 . 1699 | − 0 . 0358 − 131 . 646 − 3 . 1099   | 10 |
| A := |  | , |
|  0 . 0 | − 0 . 0002 − 1 . 5333 0 . 9281  | 10 3 |
| 0 . 0 | 0 . 0003 − 4 . 6719 − 1 . 9076 |  |
|  |  | 10 2 |
| 0 . 0 | 0 . 0 0 . 0 |  |
|   0 . 0016 | 0 . 0525 0 . 1574   | 10 1 |
| Bu := | , |  |
|  − 0 . 0 − 0 . 0031 | 0 . 0008  |  |
| 0 . 0 − 0 . 4503 | − 0 . 0614 | 10 0 |
|   |  |  |
| 0 |  | 10 -1 |
|  1  |  |  |
| Bd :=   . |  | 10 -2 |
|  0  |  |  |
| 0 |  | 10 -3 |
| The control objective is to keep perturbations in the flight- |  | Actuator |
| path angle θ − α and V t to be small due to the disturbance in |  | Fig. 3. Minimum actuator cutoff frequencies (rad/s). |
| Vt , in the H2 and H ∞ sense. Therefore, |  |  |
|  | 1 0 − 1 0 0 |  |
| Cz := W z | , Dd := | , 1 |
|  | 0 1 0 0 0 | 10 |

**C D**~**d**~ **where W**~**z**~ **:= diag 11.46 0.1 , scales z such that the desired closed-loop performance, defined by the H or H**

¯

**d norm of the transfer function from w**~**a**~

¯

**one.**

**2 ∞**

**→ z, is bounded by Results from theorems 1 and 2, will quantify the maximum actuator degradation in thrust (T), elevator (δ**~**e**~**), and leading-edge flap (δ**~**lef**~**).**

- B. **Simulation Results The numerical values for the minimum actuator cut-off frequencies and dc gains, and maximum actuator noises are shown in table I, and are plotted in Fig.3, Fig.4, and Fig.5. as**

**p**

ω<sup>**∗**</sup>

**(rad/s) kx → x**~**F**~ **k**<sup>**∗**</sup> **1/ κ**<sup>**∗**</sup> **c i ∞ i**

**Actuator H**~2~ **H**~**∞**~ **H**~2~ **H**~**∞**~ **H**~2~ **H**~**∞**~ **T 0.0033 2.4381 2.4945 0.0207 0.7157 0.0402 δ**~**e**~

**12.9757 16079.2678 16.1579 0.1707 0.0246 0.0014 δ**~**lef**~ **2.0541 2033.7134 17.8508 0.4800 0.0468 0.0027**

**TABLE I PARAMETERS THAT QUANTIFY MAXIMUM ACTUATOR DEGRADATION FOR A GIVEN CLOSED**

- **-LOOP PERFORMANCE (γ = 0.5).**

**Table 1 indicates that the control rate for thrust is sigificantly lower than that for the elevator and leading-edge flap, with the elevator requiring a higher actuation rate compared to the leading-edge flap. Furthermore, the actuation rate resulting from the H**~**2**~ **formulation is substantially lower than that from the H**~**∞**~ **formulation. This discrepancy arises because the exogenous inputs are presumed to be broad-band, and the H**~**∞**~ **formulation attenuates disturbances across this broad range. Employing a frequency-weighted model for the exogenous signal is anticipated to yield improved results and will be explored in our future work.**

**10**<sup>**0**</sup> **-1 10**

**10**<sup>**-2**</sup> **Actuator**

**Fig. 4. Minimum actuator dcgain.**

**0 10 10**<sup>**-1**</sup> **-2 10**

**-3 10 Actuator Fig. 5. Maximum actuator noise scaling.**

**The control magnitude, which is quantified by the H**~**∞**~ **norm of the actuator dynamics, is significantly higher for the H**~**2**~ **closed-loop performance than the H**~**∞**~ **closed-loop**
