# 实现计划

1. 已完成：在 TUI Config 的完整设置解析、保存和列表项中加入诊断开关（AC-1）。
2. 已完成：由 `memory` 功能 owner 管理开始、停止、失败、句柄和预算轮换；AppDriver 只调度后台请求，Status 只接收展示状态（AC-2～AC-4、INV-1）。
3. 已完成：删除 `/memory` 目录、路由、命令处理和旧测试，更新真实 TUI 场景（AC-5）。
4. 已完成 Windows 可执行的设置、运行层、Status、Slash 目录、App Server Client 和构建验证；Unix PTY 场景已编译但未在本机执行。
