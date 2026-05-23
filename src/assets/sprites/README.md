# 精灵图资源目录（已归档）

**当前不在使用**。详见 [`docs/ADR/0008-shelve-sprite-images.md`](../../../docs/ADR/0008-shelve-sprite-images.md)。

Capybit 的水豚现在永远用 `src/renderer/placeholder_capybara.js` 的 Canvas 2D 代码绘制
（含径向渐变、柔边毛感、地面阴影、10 个动作各自的姿态和小动画）。
`src/renderer/sprite.js` 顶部的 `PREFER_CODE_DRAWN = true` 开关让加载逻辑直接跳过这里的 PNG。

目录里残留的 PNG 文件是早期占位实验的产物，**不会被读取**，可以删；
保留也无害，仅占磁盘空间。

如果未来某天想 A/B 真精灵渲染（不打算这么做，但记录下方法）：

1. 把 `PREFER_CODE_DRAWN` 改为 `false`
2. 按 ADR 0008 关联的旧 PRD §6.5 命名规则补齐 10 个动作的 PNG
3. 重启 `pnpm tauri dev`，前端探测到全集会自动切换
