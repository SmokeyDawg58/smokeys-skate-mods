# Jacobian_RQD::Create raw-register audit — 2026-09-22

Read-only code audit by runtime_path_audit; root explicitly authorized this new evidence file. No production source edits and no Cargo execution performed by this agent.

## Source and method

Fresh IDA disassembly and memory bytes for native `Jacobian_RQD::Create`, `0x82ae3750..0x82ae3930` (480 bytes, 120 instructions). Source is shared by Drive and Joint Build. IDA's VX128 VA rendering is incorrect at several instructions, so arithmetic operands below use the raw instruction bits:

```text
VD = ((word >> 21) & 31) | (((word >> 2) & 3) << 5)
VA = ((word >> 16) & 31) | (((word >> 5) & 1) << 5) | (((word >> 10) & 1) << 6)
VB = ((word >> 11) & 31) | ((word & 3) << 5)
```

The symbolic evaluator loaded a/b lanes, propagated every multiply/add/subtract/OR, splat, masked insert, and store in address order. Every insertion source is splatted, so rotation does not change its inserted value. Masks 8/4/2/1 insert X/Y/Z/W respectively. Register assignments overwrite earlier expressions, including v39 and v44. The final source expressions were independently evaluated with f32 rounding at every arithmetic operation using JavaScript Math.fround. Expected values below were not copied from current Rust output. This is an ordinary-finite-input oracle; it does not claim Xenon denormal, NaN, or signed-zero exception fidelity.

## Proven defects in current shared helper

In `crates/skate2-core/src/physics/solver/simulation_constraint_math.rs::create_jacobian_rqd_rows`, only row 2 X/Y differ from native. With ax=b*a.x, aw=b*a.w, ay=b*a.y, az=b*a.z, the corrected native expressions are:

```rust
let jwk_x = ((ax[2] + az[0]) + ay[3]) + aw[1];
let jwk_y = ((az[1] + ay[2]) - aw[0]) - ax[3];
```

Current Rust reuses `jwi_y` and `jwi_x - ax[3]`, losing native register overwrites:
- 82ae3848 overwrites v39 with v51+v34; 82ae3898 inserts that new v39 into row 2 X.
- 82ae3840 overwrites v44 with v50-v35; 82ae38a8 computes v32=v44-v37; 82ae38cc inserts v32 into row 2 Y.

For identical +90-degree yaw quaternions, current row 2 XYZ collapses to zero, whereas source row 2 is approximately [1,0,0]. A zero axis permits zero effective angular mass and a reciprocal infinity; ensuing zero-times-infinity causes NaN. This is an evidence-based explanation of the observed Drive angular Z NaN; a corrected original runtime replay remains necessary to prove it fixes that execution.

## All final source symbolic rows

a0..a3 and b0..b3 denote quaternion lanes. Parentheses preserve the arithmetic ordering.

```json
[
  [
    "((((b3*a3)+(b0*a0))-(b1*a1))-(b2*a2))",
    "((((b0*a1)+(b1*a0))+(b2*a3))+(b3*a2))",
    "((((b2*a0)+(b0*a2))-(b1*a3))-(b3*a1))",
    "((((b0*a3)-(b3*a0))+(b1*a2))-(b2*a1))"
  ],
  [
    "((((b0*a1)+(b1*a0))-(b2*a3))-(b3*a2))",
    "((((b3*a3)+(b1*a1))-(b2*a2))-(b0*a0))",
    "((((b2*a1)+(b1*a2))+(b0*a3))+(b3*a0))",
    "((((b1*a3)-(b3*a1))+(b2*a0))-(b0*a2))"
  ],
  [
    "((((b2*a0)+(b0*a2))+(b3*a1))+(b1*a3))",
    "((((b1*a2)+(b2*a1))-(b0*a3))-(b3*a0))",
    "((((b3*a3)+(b2*a2))-(b0*a0))-(b1*a1))",
    "((((b2*a3)-(b3*a2))+(b0*a1))-(b1*a0))"
  ],
  [
    "((((b3*a0)-(b0*a3))+(b1*a2))-(b2*a1))",
    "((((b3*a1)-(b1*a3))+(b2*a0))-(b0*a2))",
    "((((b3*a2)-(b2*a3))+(b0*a1))-(b1*a0))",
    "((((b3*a3)+(b2*a2))+(b1*a1))+(b0*a0))"
  ]
]
```

## Independently evaluated fixed fixtures

Hex words are IEEE binary32. The original existing discriminator changes only row 2 X/Y to bd428f58 / 3dbbbbbe. Identity and same-yaw fixtures distinguish the geometric defect directly.

```json
[
  {
    "name": "existing discriminator",
    "a": [
      "3e19999a",
      "be4ccccd",
      "3e99999a",
      "3f59999a"
    ],
    "b": [
      "bdcccccd",
      "3e800000",
      "beaaaaab",
      "3f666666"
    ],
    "rows": [
      [
        "3f666667",
        "3d34e818",
        "bde6666a",
        "be58bf26"
      ],
      [
        "3d911114",
        "3f547ae1",
        "3e444446",
        "3ebeb851"
      ],
      [
        "bd428f58",
        "3dbbbbbe",
        "3f3ae147",
        "bf122224"
      ],
      [
        "3e69d038",
        "bed33333",
        "3f092c60",
        "3f199999"
      ]
    ]
  },
  {
    "name": "identity",
    "a": [
      "00000000",
      "00000000",
      "00000000",
      "3f800000"
    ],
    "b": [
      "00000000",
      "00000000",
      "00000000",
      "3f800000"
    ],
    "rows": [
      [
        "3f800000",
        "00000000",
        "00000000",
        "00000000"
      ],
      [
        "00000000",
        "3f800000",
        "00000000",
        "00000000"
      ],
      [
        "00000000",
        "00000000",
        "3f800000",
        "00000000"
      ],
      [
        "00000000",
        "00000000",
        "00000000",
        "3f800000"
      ]
    ]
  },
  {
    "name": "same +90 degree yaw",
    "a": [
      "00000000",
      "3f3504f3",
      "00000000",
      "3f3504f3"
    ],
    "b": [
      "00000000",
      "3f3504f3",
      "00000000",
      "3f3504f3"
    ],
    "rows": [
      [
        "00000000",
        "00000000",
        "bf7fffff",
        "00000000"
      ],
      [
        "00000000",
        "3f7fffff",
        "00000000",
        "00000000"
      ],
      [
        "3f7fffff",
        "00000000",
        "00000000",
        "00000000"
      ],
      [
        "00000000",
        "00000000",
        "00000000",
        "3f7fffff"
      ]
    ]
  }
]
```

## Full native instruction listing and corrected arithmetic operands

The left instruction is IDA's display. An arrow gives raw-decoded VX128 arithmetic/OR operands. Instructions without an arrow retain the printed operand form.

```text
82ae3750 13e020c7 lvx128 v63, r0, r4
82ae3754 39630010 addi r11, r3, 0x10
82ae3758 17dffaf5 vor128 v62, v95, v63 => vor128 v62, v63, v63
82ae375c 1ba0ff35 vspltw128 v61, v63, 0
82ae3760 175ffaf5 vor128 v58, v95, v63 => vor128 v58, v63, v63
82ae3764 136028c7 lvx128 v59, r0, r5
82ae3768 171ffaf5 vor128 v56, v95, v63 => vor128 v56, v63, v63
82ae376c 128018c7 lvx128 v52, r0, r3
82ae3770 39430020 addi r10, r3, 0x20 # ' '
82ae3774 1b83f735 vspltw128 v60, v62, 3
82ae3778 16fbe8b5 vmulfp128 v55, v91, v61 => vmulfp128 v55, v59, v61
82ae377c 1b21d735 vspltw128 v57, v58, 1
82ae3780 1ac2c735 vspltw128 v54, v56, 2
82ae3784 16bbe0b5 vmulfp128 v53, v91, v60 => vmulfp128 v53, v59, v60
82ae3788 167bc8b5 vmulfp128 v51, v91, v57 => vmulfp128 v51, v59, v57
82ae378c 165bb0b5 vmulfp128 v50, v91, v54 => vmulfp128 v50, v59, v54
82ae3790 1a20bf35 vspltw128 v49, v55, 0
82ae3794 19e1bf35 vspltw128 v47, v55, 1
82ae3798 1942bf35 vspltw128 v42, v55, 2
82ae379c 18a3bf35 vspltw128 v37, v55, 3
82ae37a0 1a03af35 vspltw128 v48, v53, 3
82ae37a4 19c09f35 vspltw128 v46, v51, 0
82ae37a8 19609735 vspltw128 v43, v50, 0
82ae37ac 19a19f35 vspltw128 v45, v51, 1
82ae37b0 15908835 vaddfp128 v44, v80, v49 => vaddfp128 v44, v48, v49
82ae37b4 1860af35 vspltw128 v35, v53, 0
82ae37b8 152e7835 vaddfp128 v41, v78, v47 => vaddfp128 v41, v46, v47
82ae37bc 1902af35 vspltw128 v40, v53, 2
82ae37c0 148a5835 vaddfp128 v36, v74, v43 => vaddfp128 v36, v42, v43
82ae37c4 1841af35 vspltw128 v34, v53, 1
82ae37c8 14d06835 vaddfp128 v38, v80, v45 => vaddfp128 v38, v48, v45
82ae37cc 18e29735 vspltw128 v39, v50, 2
82ae37d0 17c32875 vsubfp128 v62, v67, v37 => vsubfp128 v62, v35, v37
82ae37d4 1ba39f35 vspltw128 v61, v51, 3
82ae37d8 1be19735 vspltw128 v63, v50, 1
82ae37dc 18239735 vspltw128 v33, v50, 3
82ae37e0 14103835 vaddfp128 v32, v80, v39 => vaddfp128 v32, v48, v39
82ae37e4 1b829f35 vspltw128 v60, v51, 2
82ae37e8 1742e875 vsubfp128 v58, v66, v61 => vsubfp128 v58, v34, v61
82ae37ec 16fcf835 vaddfp128 v55, v92, v63 => vaddfp128 v55, v60, v63
82ae37f0 176c6875 vsubfp128 v59, v76, v45 => vsubfp128 v59, v44, v45
82ae37f4 17294035 vaddfp128 v57, v73, v40 => vaddfp128 v57, v41, v40
82ae37f8 16c41075 vsubfp128 v54, v68, v34 => vsubfp128 v54, v36, v34
82ae37fc 16a63875 vsubfp128 v53, v70, v39 => vsubfp128 v53, v38, v39
82ae3800 161ef835 vaddfp128 v48, v94, v63 => vaddfp128 v48, v62, v63
82ae3804 17094075 vsubfp128 v56, v73, v40 => vsubfp128 v56, v41, v40
82ae3808 1664e835 vaddfp128 v51, v68, v61 => vaddfp128 v51, v36, v61
82ae380c 165fe035 vaddfp128 v50, v95, v60 => vaddfp128 v50, v63, v60
82ae3810 153a5035 vaddfp128 v41, v90, v42 => vaddfp128 v41, v58, v42
82ae3814 14971835 vaddfp128 v36, v87, v35 => vaddfp128 v36, v55, v35
82ae3818 159b3875 vsubfp128 v44, v91, v39 => vsubfp128 v44, v59, v39
82ae381c 14f90835 vaddfp128 v39, v89, v33 => vaddfp128 v39, v57, v33
82ae3820 17d6e875 vsubfp128 v62, v86, v61 => vsubfp128 v62, v54, v61
82ae3824 17758875 vsubfp128 v59, v85, v49 => vsubfp128 v59, v53, v49
82ae3828 1750e075 vsubfp128 v58, v80, v60 => vsubfp128 v58, v48, v60
82ae382c 14d80875 vsubfp128 v38, v88, v33 => vsubfp128 v38, v56, v33
82ae3830 17295875 vsubfp128 v57, v73, v43 => vsubfp128 v57, v41, v43
82ae3834 15280875 vsubfp128 v41, v72, v33 => vsubfp128 v41, v40, v33
82ae3838 17042835 vaddfp128 v56, v68, v37 => vaddfp128 v56, v36, v37
82ae383c 1a886715 vrlimi128 v52, v44, 8, 0
82ae3840 15921875 vsubfp128 v44, v82, v35 => vsubfp128 v44, v50, v35
82ae3844 1a843fd5 vrlimi128 v52, v39, 4, 3
82ae3848 14f31035 vaddfp128 v39, v83, v34 => vaddfp128 v39, v51, v34
82ae384c 1a82f795 vrlimi128 v52, v62, 2, 2
82ae3850 1a81d755 vrlimi128 v52, v58, 1, 1
82ae3854 128019c7 stvx128 v52, r0, r3
82ae3858 12e058c7 lvx128 v55, r0, r11
82ae385c 1ae83715 vrlimi128 v55, v38, 8, 0
82ae3860 16d7baf5 vor128 v54, v87, v55 => vor128 v54, v55, v55
82ae3864 12e059c7 stvx128 v55, r0, r11
82ae3868 14c08875 vsubfp128 v38, v64, v49 => vsubfp128 v38, v32, v49
82ae386c 1ac4dfd5 vrlimi128 v54, v59, 4, 3
82ae3870 16b6b2f5 vor128 v53, v86, v54 => vor128 v53, v54, v54
82ae3874 12c059c7 stvx128 v54, r0, r11
82ae3878 1aa2c795 vrlimi128 v53, v56, 2, 2
82ae387c 1695aaf5 vor128 v52, v85, v53 => vor128 v52, v53, v53
82ae3880 12a059c7 stvx128 v53, r0, r11
82ae3884 1a81cf55 vrlimi128 v52, v57, 1, 1
82ae3888 128059c7 stvx128 v52, r0, r11
82ae388c 39630030 addi r11, r3, 0x30 # '0'
82ae3890 120050c7 lvx128 v48, r0, r10
82ae3894 14851875 vsubfp128 v36, v69, v35 => vsubfp128 v36, v37, v35
82ae3898 1a083f15 vrlimi128 v48, v39, 8, 0
82ae389c 147d1075 vsubfp128 v35, v93, v34 => vsubfp128 v35, v61, v34
82ae38a0 14414075 vsubfp128 v34, v65, v40 => vsubfp128 v34, v33, v40
82ae38a4 14206835 vaddfp128 v33, v64, v45 => vaddfp128 v33, v32, v45
82ae38a8 140c2875 vsubfp128 v32, v76, v37 => vsubfp128 v32, v44, v37
82ae38ac 171082f5 vor128 v56, v80, v48 => vor128 v56, v48, v48
82ae38b0 17c97035 vaddfp128 v62, v73, v46 => vaddfp128 v62, v41, v46
82ae38b4 120051c7 stvx128 v48, r0, r10
82ae38b8 17a66875 vsubfp128 v61, v70, v45 => vsubfp128 v61, v38, v45
82ae38bc 1764f835 vaddfp128 v59, v68, v63 => vaddfp128 v59, v36, v63
82ae38c0 17435035 vaddfp128 v58, v67, v42 => vaddfp128 v58, v35, v42
82ae38c4 17227035 vaddfp128 v57, v66, v46 => vaddfp128 v57, v34, v46
82ae38c8 16e18835 vaddfp128 v55, v65, v49 => vaddfp128 v55, v33, v49
82ae38cc 1b0407d5 vrlimi128 v56, v32, 4, 3
82ae38d0 16de7875 vsubfp128 v54, v94, v47 => vsubfp128 v54, v62, v47
82ae38d4 16b8c2f5 vor128 v53, v88, v56 => vor128 v53, v56, v56
82ae38d8 130051c7 stvx128 v56, r0, r10
82ae38dc 1aa2ef95 vrlimi128 v53, v61, 2, 2
82ae38e0 169be075 vsubfp128 v52, v91, v60 => vsubfp128 v52, v59, v60
82ae38e4 165a5875 vsubfp128 v50, v90, v43 => vsubfp128 v50, v58, v43
82ae38e8 16197875 vsubfp128 v48, v89, v47 => vsubfp128 v48, v57, v47
82ae38ec 1675aaf5 vor128 v51, v85, v53 => vor128 v51, v53, v53
82ae38f0 12a051c7 stvx128 v53, r0, r10
82ae38f4 1a61b755 vrlimi128 v51, v54, 1, 1
82ae38f8 126051c7 stvx128 v51, r0, r10
82ae38fc 122058c7 lvx128 v49, r0, r11
82ae3900 1a28a715 vrlimi128 v49, v52, 8, 0
82ae3904 15f18af5 vor128 v47, v81, v49 => vor128 v47, v49, v49
82ae3908 122059c7 stvx128 v49, r0, r11
82ae390c 19e497d5 vrlimi128 v47, v50, 4, 3
82ae3910 15cf7af5 vor128 v46, v79, v47 => vor128 v46, v47, v47
82ae3914 11e059c7 stvx128 v47, r0, r11
82ae3918 19c28795 vrlimi128 v46, v48, 2, 2
82ae391c 15ae72f5 vor128 v45, v78, v46 => vor128 v45, v46, v46
82ae3920 11c059c7 stvx128 v46, r0, r11
82ae3924 19a1bf55 vrlimi128 v45, v55, 1, 1
82ae3928 11a059c7 stvx128 v45, r0, r11
82ae392c 4e800020 blr
```

## Downstream oracle scope

Existing Drive and Joint Build golden tests consuming RQD may change, because two source expressions were wrong. Recompute expectations through an independently decoded native Build oracle; do not update goldens merely to accept production output. The fixed RQD fixtures above establish only this 480-byte routine, not complete downstream Build coefficients or VMX estimate behavior.

