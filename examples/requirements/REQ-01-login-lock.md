---
id: REQ-01
title: 登录暴力破解防护
type: requirement
priority: high
status: draft
tags: [security, auth]
owner: "@security-team"
---

# 登录暴力破解防护

为了防止恶意用户通过字典攻击猜解密码，系统必须具备账户锁定机制。

## 业务背景

当用户连续输入错误密码时，系统应暂时冻结该账户。

## 验收标准

### AC-01: 触发锁定

- **Given**: 账户处于正常状态
- **When**: 用户在 5 分钟内连续输错密码 5 次
- **Then**: 账户被锁定 30 分钟
- **And**: 向用户发送安全警告邮件

### AC-02: 锁定期间尝试

- **Given**: 账户已被锁定
- **When**: 用户输入**正确**的密码
- **Then**: 拒绝登录，并提示"账户锁定中"
