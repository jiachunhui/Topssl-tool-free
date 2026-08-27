---
title: "ARI正式成为RFC 9773：ACME续期信息机制如何重塑证书自动续期"
description: "Let's Encrypt主导的ACME Renewal Information（ARI）正式发布为RFC 9773。解析renewalInfo端点原理、防惊群设计、客户端支持现状与站长升级建议，一文读懂未来续期调度机制。"
pubDate: 2026-09-05
category: 技术动态
tags:
  - ARI
  - RFC 9773
  - ACME续期
  - SSL证书自动续期
  - Let's Encrypt
related:
  - ssl-auto-renewal-guide
  - lets-encrypt-6day-certs
  - lets-encrypt-90day-renewal
---

## ARI 是什么

ARI 全称 ACME Renewal Information（ACME 续期信息），是 Let's Encrypt 主导设计、用于优化 ACME 证书自动续期调度的一项机制。2025 年 9 月 16 日，Let's Encrypt 宣布 ARI 正式发布为互联网标准 [RFC 9773](https://letsencrypt.org/2025/09/16/ari-rfc.html)，从"Let's Encrypt 自家功能"升级为全行业通用协议。

ARI 解决的核心问题是：**什么时候续期，不该由客户端自己拍脑袋，而应由 CA 统一调度**。传统续期逻辑是客户端在证书到期前固定天数（如 30 天）自行发起续期，全网数百万证书在同一窗口扎堆请求，既造成 CA 服务器压力，也无法应对突发情况（比如某 CA 突然需要大规模吊销证书）。

## renewalInfo 端点的工作原理

ARI 在 ACME 协议中新增了一个 `renewalInfo` 端点，工作流程如下：

1. 客户端解析证书中的 ARI 扩展（或直接向 CA 查询），拿到 `renewalInfo` 地址；
2. 到期前客户端调用该端点，CA 返回一个**建议续期时间窗口**（包含开始与结束时间）；
3. CA 会在窗口内加入随机抖动，把全网证书的续期请求均匀打散，避免"惊群效应"（thundering herd）；
4. 客户端在窗口内完成续期，成功后不再重复请求。

这套机制对未来的短有效期证书至关重要——6 天证书、45 天证书（详见[6 天证书详解](/blog/lets-encrypt-6day-certs/)与[45 天政策解读](/blog/lets-encrypt-45day-policy/)）之所以能规模化签发，正是因为 ARI 让高频续期变得有序可控。

## ARI 的应急能力：CA 的"紧急刹车"

ARI 还有一个普通站长容易忽略的价值：**应急续期调度**。当 CA 遭遇私钥泄露、系统漏洞等需要提前吊销证书的危机时，可以通过 ARI 向所有客户端推送"立即续期"信号，让海量证书在短时间内完成轮换，把损失窗口压缩到最小。这是传统固定天数续期完全做不到的——传统模式下，CA 只能眼睁睁等证书自然到期。

## 客户端支持现状

| 客户端 | ARI 支持情况 |
| --- | --- |
| certbot | 3.x 版本已支持，会自动查询 renewalInfo |
| acme.sh | 3.1.4 及以上版本已支持（[发布说明](https://github.com/acmesh-official/acme.sh/releases/tag/3.1.4)） |
| ToSSL | 内置 ARI 适配，续期窗口跟随 CA 调度 |
| 其他 ACME 客户端 | 请查阅各自更新日志，确认是否实现 RFC 9773 |

## 站长现在该做什么

1. **升级客户端**：确认 certbot / acme.sh / 图形化工具为支持 ARI 的新版本，这是未来 45 天、6 天证书时代的前提。
2. **验证 ARI 是否生效**：检查续期日志，能看到 renewalInfo 请求与建议窗口的输出即说明已生效；老版本客户端会退化为固定天数续期，短证书时代将无法使用。
3. **保持自动续期常驻**：ARI 是"锦上添花"的调度优化，前提依然是你的续期链路本身可靠。用 ToSSL 的话，保持应用后台运行并开启开机自启即可，续期窗口会由 CA 自动安排。

## 常见问题

- **ARI 是强制要求吗？** 签发 6 天证书时是硬性要求；常规 90 天证书目前仍兼容非 ARI 客户端，但随着行业走向 45 天，ARI 支持将成为标配。
- **ARI 会影响证书价格吗？** 不会，ARI 是协议层面的改进，不涉及任何费用。
- **我的证书需要手动配置 ARI 吗？** 不需要。只要客户端版本支持，续期流程全自动，你无需改动任何配置。
- **ARI 和自动续期是什么关系？** 自动续期是"到点去续"的执行动作，ARI 是"什么时候续"的调度决策，两者配合才能支撑短有效期证书的大规模落地。

想搭建完整的自动续期体系，可参考站内[三种自动续期方案对比](/blog/ssl-auto-renewal-guide/)与[自动续期配置文档](/docs/auto-renewal/)。
