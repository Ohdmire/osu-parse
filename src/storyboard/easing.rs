//! osu! storyboard 缓动函数。
//!
//! 编号与公式**逐分支对照 osu!framework(lazer)**:
//! - 枚举顺序 `osu.Framework/Graphics/Easing.cs`:26/27 为
//!   `OutElasticHalf`/`OutElasticQuarter`(自 2016 年 initial commit 起插在
//!   `OutElastic` 之后),26 号起比 easings.net 的常见顺序偏移 +2,
//!   SB 文件里的缓动数字在 lazer 中被直接 cast 到该枚举
//!   (`LegacyStoryboardDecoder`: `(Easing)Parsing.ParseInt(split[1])`);
//! - 公式 `osu.Framework/Graphics/Transforms/DefaultEasingFunction.cs`,
//!   含 expo/elastic 的端点偏移修正(t=0/1 精确落在 0/1)。

use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Easing(pub i32);

impl Easing {
    pub const LINEAR: Easing = Easing(0);

    /// 未知的缓动 id 回退为线性(framework switch 的 default 分支)。
    pub fn from_id(id: i32) -> Easing {
        if (0..=35).contains(&id) {
            Easing(id)
        } else {
            Easing(0)
        }
    }

    /// 输入 clamp 到 [0,1],输出为插值进度。
    pub fn apply(self, time: f32) -> f32 {
        let t = time.clamp(0.0, 1.0);
        match self.0 {
            0 => t,
            1 | 4 => t * (2.0 - t), // Out / OutQuad
            2 | 3 => t * t,         // In / InQuad
            5 => {
                // InOutQuad
                if t < 0.5 {
                    t * t * 2.0
                } else {
                    let u = t - 1.0;
                    u * u * -2.0 + 1.0
                }
            }
            6 => t * t * t, // InCubic
            7 => {
                let u = t - 1.0;
                u * u * u + 1.0 // OutCubic
            }
            8 => {
                // InOutCubic
                if t < 0.5 {
                    t * t * t * 4.0
                } else {
                    let u = t - 1.0;
                    u * u * u * 4.0 + 1.0
                }
            }
            9 => t.powi(4), // InQuart
            10 => {
                let u = t - 1.0;
                1.0 - u * u * u * u // OutQuart
            }
            11 => {
                // InOutQuart
                if t < 0.5 {
                    t.powi(4) * 8.0
                } else {
                    let u = t - 1.0;
                    u * u * u * u * -8.0 + 1.0
                }
            }
            12 => t.powi(5), // InQuint
            13 => {
                let u = t - 1.0;
                u * u * u * u * u + 1.0 // OutQuint
            }
            14 => {
                // InOutQuint
                if t < 0.5 {
                    t.powi(5) * 16.0
                } else {
                    let u = t - 1.0;
                    u * u * u * u * u * 16.0 + 1.0
                }
            }
            15 => 1.0 - (t * PI * 0.5).cos(), // InSine
            16 => (t * PI * 0.5).sin(),       // OutSine
            17 => 0.5 - 0.5 * (PI * t).cos(), // InOutSine
            18 => {
                // InExpo(端点修正)
                let off = 2.0f32.powf(-10.0);
                2.0f32.powf(10.0 * (t - 1.0)) + off * (t - 1.0)
            }
            19 => {
                // OutExpo(端点修正)
                let off = 2.0f32.powf(-10.0);
                -2.0f32.powf(-10.0 * t) + 1.0 + off * t
            }
            20 => {
                // InOutExpo(端点修正)
                let off = 2.0f32.powf(-10.0);
                if t < 0.5 {
                    0.5 * (2.0f32.powf(20.0 * t - 10.0) + off * (2.0 * t - 1.0))
                } else {
                    1.0 - 0.5 * (2.0f32.powf(-20.0 * t + 10.0) + off * (-2.0 * t + 1.0))
                }
            }
            21 => 1.0 - (1.0 - t * t).sqrt(), // InCirc
            22 => {
                let u = t - 1.0;
                (1.0 - u * u).sqrt() // OutCirc
            }
            23 => {
                // InOutCirc
                let u = t * 2.0;
                if u < 1.0 {
                    0.5 - 0.5 * (1.0 - u * u).sqrt()
                } else {
                    let v = u - 2.0;
                    0.5 * (1.0 - v * v).sqrt() + 0.5
                }
            }
            24 => {
                // InElastic(framework 公式,带端点修正)
                let (c, c2) = (2.0 * PI / 0.3, 0.3 / 4.0);
                let off = 2.0f32.powf(-11.0);
                -2.0f32.powf(-10.0 + 10.0 * t) * ((1.0 - c2 - t) * c).sin() + off * (1.0 - t)
            }
            25 => {
                // OutElastic(framework 公式,带端点修正)
                let (c, c2) = (2.0 * PI / 0.3, 0.3 / 4.0);
                let off = 2.0f32.powf(-11.0);
                2.0f32.powf(-10.0 * t) * ((t - c2) * c).sin() + 1.0 - off * t
            }
            26 => {
                // OutElasticHalf
                let (c, c2) = (2.0 * PI / 0.3, 0.3 / 4.0);
                let off = 2.0f32.powf(-10.0) * ((0.5 - c2) * c).sin();
                2.0f32.powf(-10.0 * t) * ((0.5 * t - c2) * c).sin() + 1.0 - off * t
            }
            27 => {
                // OutElasticQuarter
                let (c, c2) = (2.0 * PI / 0.3, 0.3 / 4.0);
                let off = 2.0f32.powf(-10.0) * ((0.25 - c2) * c).sin();
                2.0f32.powf(-10.0 * t) * ((0.25 * t - c2) * c).sin() + 1.0 - off * t
            }
            28 => {
                // InOutElastic(framework 专用公式)
                let (c, c2) = (2.0 * PI / 0.3, 0.3 / 4.0);
                let off = 2.0f32.powf(-10.0) * ((1.0 - c2 * 1.5) * c / 1.5).sin();
                let u = t * 2.0;
                if u < 1.0 {
                    -0.5 * (2.0f32.powf(-10.0 + 10.0 * u) * ((1.0 - c2 * 1.5 - u) * c / 1.5).sin()
                        - off * (1.0 - u))
                } else {
                    let v = u - 1.0;
                    0.5 * (2.0f32.powf(-10.0 * v) * ((v - c2 * 1.5) * c / 1.5).sin() - off * v)
                        + 1.0
                }
            }
            29 => {
                // InBack
                let bc = 1.70158;
                t * t * ((bc + 1.0) * t - bc)
            }
            30 => {
                // OutBack
                let bc = 1.70158;
                let u = t - 1.0;
                u * u * ((bc + 1.0) * u + bc) + 1.0
            }
            31 => {
                // InOutBack(back_const2 = back_const * 1.525)
                let bc2 = 1.70158f32 * 1.525;
                let u = t * 2.0;
                if u < 1.0 {
                    0.5 * u * u * ((bc2 + 1.0) * u - bc2)
                } else {
                    let v = u - 2.0;
                    0.5 * (v * v * ((bc2 + 1.0) * v + bc2) + 2.0)
                }
            }
            32 => {
                // InBounce(framework 逐分支)
                let u = 1.0 - t;
                1.0 - bounce_out(u)
            }
            33 => bounce_out(t), // OutBounce
            34 => {
                // InOutBounce
                if t < 0.5 {
                    0.5 - 0.5 * bounce_out(1.0 - t * 2.0)
                } else {
                    bounce_out((t - 0.5) * 2.0) * 0.5 + 0.5
                }
            }
            35 => {
                // OutPow10:1 - (1-t)^11
                let u = t - 1.0;
                u * u.powi(10) + 1.0
            }
            _ => t,
        }
    }
}

/// framework `OutBounce` 分段(常量 7.5625 / 1/2.75)。
fn bounce_out(t: f32) -> f32 {
    let bc = 1.0 / 2.75;
    if t < bc {
        7.5625 * t * t
    } else if t < 2.0 * bc {
        let u = t - 1.5 * bc;
        7.5625 * u * u + 0.75
    } else if t < 2.5 * bc {
        let u = t - 2.25 * bc;
        7.5625 * u * u + 0.9375
    } else {
        let u = t - 2.625 * bc;
        7.5625 * u * u + 0.984375
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_are_0_and_1() {
        for id in 0..=35 {
            let e = Easing(id);
            let a = e.apply(0.0);
            let b = e.apply(1.0);
            assert!((a - 0.0).abs() < 1e-4, "easing {id} at 0 -> {a}");
            assert!((b - 1.0).abs() < 1e-4, "easing {id} at 1 -> {b}");
        }
    }

    #[test]
    fn clamps_input() {
        assert_eq!(Easing::LINEAR.apply(-1.0), 0.0);
        assert_eq!(Easing::LINEAR.apply(2.0), 1.0);
    }

    #[test]
    fn known_values() {
        let e = Easing(3); // InQuad
        assert!((e.apply(0.5) - 0.25).abs() < 1e-5);
        let e = Easing(4); // OutQuad
        assert!((e.apply(0.5) - 0.75).abs() < 1e-5);
        let e = Easing(0);
        assert!((e.apply(0.25) - 0.25).abs() < 1e-5);
    }

    #[test]
    fn unknown_id_falls_back_to_linear() {
        assert_eq!(Easing::from_id(99), Easing::LINEAR);
        assert_eq!(Easing::from_id(-3), Easing::LINEAR);
    }

    /// framework 枚举语义抽查(编号含 26/27 插位)。
    #[test]
    fn framework_enum_alignment() {
        // 30(OutBack):中途超过 1 再回落(t≈0.7 处 >1.05)
        let e30 = Easing(30).apply(0.7);
        assert!(e30 > 1.05, "OutBack 应中途过冲: {e30}");
        // 32(InBounce)在 bounce 平台间有谷底(非单调,约 t=0.15..0.3)
        let b = [Easing(32).apply(0.15), Easing(32).apply(0.22), Easing(32).apply(0.3)];
        assert!((b[1] - b[0]) * (b[2] - b[1]) < 0.0, "InBounce 应振荡: {b:?}");
        // 33(OutBounce)在第二段中点(t≈0.545)有谷底
        let c = [Easing(33).apply(0.45), Easing(33).apply(0.545), Easing(33).apply(0.65)];
        assert!((c[1] - c[0]) * (c[2] - c[1]) < 0.0, "OutBounce 应振荡: {c:?}");
        // 25/27 在 t≈0.45 过冲;26(Half,频率减半)的首个过冲更晚(t≈0.9)
        assert!(Easing(25).apply(0.45) > 1.0, "OutElastic 应过冲");
        assert!(Easing(27).apply(0.45) > 1.0, "OutElasticQuarter 应过冲");
        assert!(Easing(26).apply(0.9) > 1.0, "OutElasticHalf 应过冲");
    }

    /// 端点修正抽查:expo/elastic 家族在端点的导出值精确(framework 的
    /// offset 修正保证),且极近端点处不越过 0/1 太多(elastic 允许过冲)。
    #[test]
    fn expo_elastic_endpoint_correction() {
        // InExpo(18):无修正的标准公式在 t=1 处 = 1 - 2^-10;修正后精确 1
        assert!((Easing(18).apply(1.0) - 1.0).abs() < 1e-6);
        assert!((Easing(18).apply(0.0)).abs() < 1e-6);
        // OutElastic(25)在 t=0 精确 0(标准公式 sin(-π/2)=-1 也为 0,
        // 但 t=1 处标准公式 = 1-2^-10·ε,修正后精确)
        assert!((Easing(25).apply(1.0) - 1.0).abs() < 1e-6);
        assert!((Easing(24).apply(0.0)).abs() < 1e-6);
        assert!((Easing(24).apply(1.0) - 1.0).abs() < 1e-6);
        assert!((Easing(28).apply(0.0)).abs() < 1e-6);
        assert!((Easing(28).apply(1.0) - 1.0).abs() < 1e-6);
    }
}
