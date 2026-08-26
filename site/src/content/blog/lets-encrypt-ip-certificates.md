---
title: "Let's Encrypt IP地址证书全面开放：没有域名也能部署HTTPS"
description: "Let's Encrypt IP地址证书（IP SAN）正式GA：公网IP无需域名即可申请免费SSL证书。详解申请条件、验证方式、适用场景与浏览器兼容性，附Tossl申请指引。"
pubDate: 2026-09-08
category: 技术动态
tags:
  - IP证书
  - IP地址SSL证书
  - Let's Encrypt
  - HTTPS加密
  - 无域名证书
related:
  - lets-encrypt-6day-certs
  - dns-validation-tutorial
  - free-ssl-cert-application-process
---

## 什么是 IP 地址证书

IP 地址证书（IP SAN 证书）是把**公网 IP 地址**作为证书主体（Subject Alternative Name）的 SSL 证书，证书不再绑定域名，而是直接绑定一个或多个 IP。2026 年 1 月 15 日，Let's Encrypt 在[官方博客](https://letsencrypt.org/2026/01/15/6day-and-ip-general-availability)宣布 IP 地址证书正式全面开放（GA）——这是 Let's Encrypt 自 2025 年初[预告该能力](https://letsencrypt.org/2025/01/16/6-day-and-ip-certs)以来的最终落地。

在此之前，主流免费 CA 只对域名签发证书，没有域名就只能用自签名证书（浏览器会报"不安全"）或付费 IP 证书。IP 地址证书的开放，补齐了"无域名场景"的 HTTPS 拼图。

## 申请条件与验证方式

申请 IP 地址证书的核心要求是：**证明你对这个公网 IP 拥有控制权**。验证方式与域名证书一一对应：

| 验证方式 | 说明 | 适用场景 |
| --- | --- | --- |
| HTTP-01 | 在 IP 的 80 端口放置验证文件 | 可直接访问的 Web 服务 |
| TLS-ALPN-01 | 在 IP 的 443 端口完成 TLS 挑战 | 支持 ALPN 的服务 |
| DNS-01 | 为 IP 反查记录添加 TXT 记录 | 无法直接暴露端口的场景 |

具体到签发条件：IP 必须是**公网地址**（私有 IP 如 192.168.x.x、10.x.x.x 不签发），支持 IPv4 与 IPv6；同一张证书可以同时包含域名和 IP，也可以只包含 IP。验证的自动化程度与域名证书一致，[DNS 验证的完整教程](/blog/dns-validation-tutorial/)同样适用。

## IP 地址证书的适用场景

- **无域名的 API 与微服务**：内部系统、开放接口直连 IP，域名缺失但有加密刚需
- **IoT 设备与硬件**：摄像头、传感器、路由器管理页面，设备往往只有 IP 没有域名
- **NAS 与自建服务**：群晖、TrueNAS 等设备的公网远程访问
- **CDN 回源与负载均衡**：源站使用 IP 直连，回源链路需要加密
- **穿透与测试环境**：临时公网服务快速上 HTTPS，省去域名申请步骤

## 浏览器兼容性说明

IP 地址证书能否被浏览器信任，取决于两点：证书本身由受信 CA 签发，且浏览器支持 IP SAN 校验。现代 Chrome、Edge、Firefox、Safari 均支持公网 IP 证书的校验；个别旧版本浏览器或部分移动端 WebView 对 IP 证书支持不完整，建议在目标用户环境实测。Let's Encrypt 官方建议将 IP 证书主要用于自动化程度高的服务场景，浏览器兼容以官方文档为准。

## 用 Tossl 申请 IP 地址证书

Tossl 已适配 Let's Encrypt 的 IP 证书签发能力：在应用内选择"IP 证书"类型，填入公网 IP，按提示完成 HTTP 或 DNS 验证即可申请，到期前同样自动续期。配合 Tossl 的部署包导出功能，可以把证书文件与 nginx / Apache 配置示例直接带到服务器完成部署。

## 常见问题

- **IP 证书免费吗？** 免费，属于 Let's Encrypt 常规签发服务，只有 DV 级别（验证域名/IP 控制权），不提供企业身份验证。
- **能申请内网 IP 的证书吗？** 不能。Let's Encrypt 只对公网 IP 签发，内网地址请使用自签名证书或企业内网 CA。
- **证书可以同时包含域名和 IP 吗？** 可以。多域名/IP 的 SAN 证书一张即可覆盖，适用多入口业务。
- **IP 变了怎么办？** 重新申请并部署新证书即可；若 IP 长期不变，配合自动续期即可稳定运行。

如果你想了解普通域名证书的完整申请流程，可阅读[免费 SSL 证书申请完整流程](/blog/free-ssl-cert-application-process/)；IP 证书与 6 天短证书同为 2026 年初同期 GA 的新能力，两者的组合使用思路可参考[6 天证书详解](/blog/lets-encrypt-6day-certs/)。
