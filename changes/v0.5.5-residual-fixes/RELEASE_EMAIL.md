# Ruyi v0.5.5 Release — Email Notification (Pending Send)

**Status**: 邮件未发送（环境缺少 `RESEND_API_KEY`）
**Created**: 2026-07-10
**Protocol**: Resend API（按 AGENTS.md / .opencode/AGENTS.md 规范）

---

## 邮件目标

- **To**: `feather.lzg@foxmail.com`
- **From**: `Ruyi Agent <onboarding@resend.dev>`
- **Subject**: `[Ruyi] 计划完成: v0.5.5 已发布`
- **Idempotency Key**: `plan-v0.5.5-release-end-2026-07-10`

## 邮件正文（HTML）

```html
<h2>Ruyi v0.5.5 已发布</h2>
<p>计划: v0.5.5-residual-fixes</p>
<p>完成时间: 2026-07-10</p>
<p>完成任务: 7/7 P0 缺陷全面解决</p>
<p>commits landed: 51 (含 1 merge commit on main + 1 roadmap meta)</p>
<p>tag: <code>v0.5.5</code> @ <code>a87ca50</code> (annotated)</p>
<p>验证结果: 229 lib tests passed; 0 net new clippy warnings (-2 fixed); 36/41 examples typecheck (5 pre-existing accepted)</p>

<h3>P0 缺陷解决清单</h3>
<ul>
  <li>✅ 异常处理 (1.7): try/catch/finally cross-function landing pad</li>
  <li>✅ 链接运行时 (2.1): 静态链接 ruyi_runtime</li>
  <li>✅ GC 分配 (2.2): 双模式 (--gc=stub|real)</li>
  <li>✅ async 真正异步 (2.3): 工作窃取调度器</li>
  <li>✅ spawn 内建 (2.4): 绿色线程</li>
  <li>✅ 异常 landing pad (2.5): invoke + landingpad LLVM IR</li>
  <li>✅ Trait 约束 (3.1): check_bounds validates impl</li>
</ul>

<h3>修改文件列表 (top 10)</h3>
<ol>
  <li>crates/ruyic/src/codegen/expr.rs (invoke + landing pad)</li>
  <li>crates/ruyic/src/codegen/gc_alloc.rs (new dispatcher)</li>
  <li>crates/ruyic/src/cli/gc_mode.rs (new)</li>
  <li>crates/ruyic/src/driver.rs (--gc=wire + DEV-001)</li>
  <li>crates/ruyic/src/typechecker/impl_table.rs (O(1) trait impl)</li>
  <li>crates/ruyic/src/typechecker/generics.rs (check_bounds)</li>
  <li>crates/ruyi_runtime/src/c_exports.rs (cc_alloc stub)</li>
  <li>crates/ruyi_runtime/src/async_runtime.rs (work-stealing scheduler)</li>
  <li>crates/ruyi_runtime/tests/spawn.rs (new 4 tests)</li>
  <li>examples/{async_sleep,try_catch_invoke,spawn_demo}.ry (new)</li>
</ol>

<h3>下一步</h3>
<p>v0.5.5 release 已 ship。建议陛下开 v0.6 包管理器变更（路线图阶段二）。</p>
```

## 发送方法

如需发送（需 RESEND_API_KEY）：

```bash
# 设置 API key（一次性）
export RESEND_API_KEY=re_xxxxxxxxxx

# 发送
curl -X POST 'https://api.resend.com/emails' \
  -H "Authorization: Bearer $RESEND_API_KEY" \
  -H 'Content-Type: application/json' \
  -d @- <<JSON
{
  "from": "Ruyi Agent <onboarding@resend.dev>",
  "to": ["feather.lzg@foxmail.com"],
  "subject": "[Ruyi] 计划完成: v0.5.5 已发布",
  "html": "...",
  "headers": { "Idempotency-Key": "plan-v0.5.5-release-end-2026-07-10" }
}
JSON
```

## 当前跳过原因

- AGENTS.md 规范：邮件失败不阻塞计划执行
- v0.5.5 release 本身已成功（merge + tag + push 全部完成）
- 邮件是 supplementary notification，非 release 阻塞项
