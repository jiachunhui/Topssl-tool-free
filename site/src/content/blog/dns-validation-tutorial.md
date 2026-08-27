---
title: "DNS验证SSL证书教程：TXT记录申请Let's Encrypt证书详解"
description: 'DNS验证SSL证书教程：DNS-01验证原理、_acme-challenge TXT记录添加步骤、阿里云/DNSPod/Cloudflare API自动验证配置与验证失败排查方法。'
pubDate: 2026-08-24
category: DNS验证
tags:
  - DNS验证
  - SSL证书
  - TXT记录
related:
  - wildcard-cert-application
  - free-ssl-cert-application-process
---

## DNS 验证是什么

DNS 验证（DNS-01）是 Let's Encrypt 证明"你确实拥有这个域名"的一种方式。它的原理是：证书颁发机构（CA）生成一个随机校验值，要求你把它写到域名的 DNS 记录里，具体是一条名为 `_acme-challenge.你的域名` 的 TXT 记录。CA 随后查询这条记录，只要能读到匹配的校验值，就认为你拥有该域名的控制权并签发证书。验证完成后，这条 TXT 记录可以删除，因为它只在验证那一刻被读取，之后不再有用途。与 HTTP 验证相比，DNS 验证不依赖服务器上的任何文件或端口，因此适用范围更广。

## 适用场景

DNS 验证在以下场景几乎是必选项：

- 80 端口不可用：服务器防火墙未放行 80 端口，或已被其他程序占用，HTTP 验证无法完成。
- 内网/无公网服务器：证书对应的服务器不直接暴露在公网，CA 无法回连。
- 通配符证书：申请 `*.example.com` 这类通配符证书时，Let's Encrypt 只接受 DNS 验证，HTTP 验证做不到。
- 域名解析不在同一服务器：域名由第三方解析托管，改 DNS 记录比动服务器更简单。

## 手动添加 TXT 记录步骤

手动验证的流程如下：

1. 在申请工具里发起申请，工具会给出记录名称与记录值，名称形如 `_acme-challenge.example.com`。
2. 登录域名服务商后台，进入 DNS 解析（或记录管理）页面，新增一条 TXT 类型的记录。
3. 将记录值完整粘贴到记录内容里，注意不要漏掉首尾或误加空格。
4. 保存后等待解析生效。多数服务商几分钟内生效，但受 TTL 影响可能更久。
5. 回到工具点击"继续验证"，由 CA 查询记录并完成签发。

## API 自动验证

手动加记录适合一次性申请，但每次续期都要重复一遍就太繁琐了。API 自动验证的思路是：你把域名服务商的 API 密钥交给工具，工具在验证时自动创建 TXT 记录、验证完成后再自动删除。以 ToSSL 为例，它支持阿里云 DNS、DNSPod、Cloudflare 等主流解析商，密钥只保存在本机，且仅用于添加与删除 `_acme-challenge` 这条 TXT 记录，不会动其他解析。

| 解析商 | 所需凭证 | 所需权限 |
| --- | --- | --- |
| 阿里云 DNS | AccessKey ID 与 Secret | 域名解析（DNS）读写 |
| DNSPod | API Token（ID 与 Token） | 解析记录读写 |
| Cloudflare | API Token 或 Global API Key | Zone:DNS:Edit |

配置好 API 后，DNS 验证就能像 HTTP 验证一样自动完成，续期也不再需要人工介入。详细的配置方式见 [DNS 验证文档](/docs/dns-validation/)。

## 验证失败排查

| 失败原因 | 表现 | 处理方向 |
| --- | --- | --- |
| 密钥无权限/过期 | 鉴权报错 | 重新授权或更换密钥 |
| 记录未生效 | 查询不到 TXT 记录 | 等待生效或检查 TTL |
| 记录值填错 | 校验不匹配 | 核对粘贴是否完整 |
| CNAME 冲突 | 主记录类型冲突 | 改用子域或调整记录 |
| 触发速率限制 | 频繁验证被拒 | 停止重试，等待窗口 |

更多排查细节可参考 [DNS 验证问题排查文档](/docs/troubleshooting/dns-validation/)。

## DNS 验证与 HTTP 验证对比

| 维度 | DNS 验证 | HTTP 验证 |
| --- | --- | --- |
| 验证方式 | 添加 TXT 记录 | 80 端口放验证文件 |
| 是否需要公网服务器 | 否 | 是（需 80 端口可达） |
| 通配符证书 | 支持（唯一方式） | 不支持 |
| 自动化 | 需解析商 API | 需服务器可写 |
| 生效速度 | 受 DNS 传播影响 | 通常更快 |

如果你的目标是申请通配符证书，可进一步阅读 [通配符证书申请指南](/blog/wildcard-cert-application/)。
