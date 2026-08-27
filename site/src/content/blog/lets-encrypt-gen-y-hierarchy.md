---
title: "Let's Encrypt启用Generation Y新根证书：ISRG Root Y1/Y2与ECDSA趋势解读"
description: "Let's Encrypt公布并完成Generation Y新根证书层级切换（2026年5月）。解读ISRG Root Y1/Y2新架构、交叉信任、旧客户端兼容期与站长需要做的准备。"
pubDate: 2026-09-12
category: 技术动态
tags:
  - 根证书
  - ISRG Root
  - Generation Y
  - ECDSA证书
  - Let's Encrypt
related:
  - lets-encrypt-security
  - lets-encrypt-45day-policy
  - post-quantum-tls
---

## 什么是 Generation Y 新根证书

2025 年 11 月 24 日，Let's Encrypt 在[官方博客](https://letsencrypt.org/2025/11/24/gen-y-hierarchy)公布了全新的"Generation Y"根证书层级。这是 Let's Encrypt 自 2015 年上线以来，首次系统性更换根证书架构：新层级包含全新的 ECDSA 与 RSA 根证书（ISRG Root Y1、ISRG Root Y2），并配套新一代中间证书，替代服役多年的 X 系列层级。

按照官方公布的时间表，切换已于 2026 年 5 月完成：X 系列层级停止签发新证书，新签发的证书全部走 Generation Y 链路；旧版客户端的兼容期延续到 2026 年 7 月后结束（据[行业报道](https://webhosting.today/2026/04/15/lets-encrypt-changes-its-root-certificates-on-may-13-client-auth-ends-july-8/)，根证书切换于 2026 年 5 月 13 日完成，客户端认证支持于 2026 年 7 月 8 日结束）。

## 为什么要在服役十年后更换根证书

根证书是浏览器信任链的源头，更换是一件极其谨慎的大事，Let's Encrypt 的动因主要有三点：

1. **密钥轮换的行业惯例**：长期使用同一根密钥存在密钥老化风险，周期性更换根证书、配套根密钥仪式（Root Ceremony），是大型 CA 的行业通行实践。
2. **服务 X 系列层级的长期维护**：X 系列层级包含多个中间证书与交叉签名组合，架构随时间变得复杂，新层级让证书链更简洁、可维护。
3. **面向 ECDSA 与后量子时代的演进**：新层级强化了 ECDSA 证书的签发能力，为更短的证书有效期（45 天）、更高频的签发做好了架构准备（见[45 天政策解读](/blog/lets-encrypt-45day-policy/)）。

## 新层级架构与兼容性

| 项目 | X 系列（旧） | Generation Y（新） |
| --- | --- | --- |
| 根证书 | ISRG Root X1（RSA）、X2（ECDSA） | ISRG Root Y1（ECDSA）、Y2（RSA） |
| 中间证书 | R3/R4、E1/E5 等 | 新一代 ECDSA / RSA 中间证书 |
| 签发状态 | 2026 年 5 月起停止新签发 | 全部新证书 |

**对普通站长来说，绝大多数情况无需任何操作**：现代操作系统与浏览器会自动信任新根证书（Y 系列根已通过预置与交叉签名进入主流信任库），服务器只要使用新签发的证书链即可。唯一需要注意的场景是**老旧客户端**（如长期未更新的嵌入式设备、老版本系统）——它们在兼容期结束后可能无法验证新证书链，需要提前升级。

## 站长现在该做什么

1. **确认证书链完整**：部署时使用 Let's Encrypt 签发的完整证书链（fullchain），不要只传证书本体，确保新旧链路都能被验证。
2. **更新 ACME 客户端**：certbot 3.x、acme.sh 3.1.4+ 以及 ToSSL 等客户端已适配新层级，升级后自动签发新链路的证书。
3. **排查老旧终端**：如果业务涉及老系统、老设备访问，建议在兼容期结束后验证一次证书信任情况，必要时为这些终端单独保留旧证书方案。
4. **顺带关注 ECDSA 证书**：新层级强化了 ECDSA 签发，ECDSA 证书密钥更短、握手更快，新建证书时可优先考虑（现代浏览器与系统均兼容）。

## 常见问题

- **新根证书需要手动安装吗？** 不需要。新根通过系统更新与交叉签名进入信任库，浏览器自动信任。
- **旧证书还能用吗？** 能用。2026 年 5 月前签发的证书在有效期内继续受信任，只是到期后不再签发 X 系列链路的新证书。
- **交叉签名是什么？** 让新根证书被旧根证书"背书"的技术，保证兼容期内的客户端能通过旧信任路径验证新证书，实现平滑过渡。
- **更换根证书会影响证书价格吗？** 不影响，免费证书依然免费。

想了解免费证书的信任体系与安全性设计，可阅读[Let's Encrypt 免费证书安全吗](/blog/lets-encrypt-security/)；新根层级与后量子密码升级的关联，可参考[后量子加密时代解读](/blog/post-quantum-tls/)。
