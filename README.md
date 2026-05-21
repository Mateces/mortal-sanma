# mortal-sanma

三人麻将（三麻）版 [Mortal](https://github.com/Equim-chan/Mortal) libriichi。

将 Mortal 的四人麻将引擎改造为三人麻将专用，可用于 AI 训练和推理。

## 与 Mortal 的区别

| 项目 | Mortal (四麻) | mortal-sanma (三麻) |
|------|--------------|-------------------|
| 玩家数 | 4 | 3 |
| 牌山 | 136 张 | 108 张（无 2-8m） |
| 红宝牌 | 5mr/5pr/5sr | 5pr/5sr（无红5m） |
| 吃 | 有 | 禁用 |
| 拔北 | 无 | Nukidora 事件 |
| 自摸损 | 无 | 缺席玩家不付 |
| 半庄 | 8 局 | 6 局（东1-3 + 南1-3） |
| ACTION_SPACE | 46 | 44 |
| obs_shape | (1012, 34) | (775, 34) |

## 兼容性

- **与 Akagi/MahjongCopilot 的 libriichi3p 接口兼容**（obs_shape=775, ACTION_SPACE=44）
- **不兼容四麻**——这是纯三麻专用 fork
- 可直接替换 MahjongCopilot 的 `libriichi3p/` 目录中的 .so/.pyd 文件

## 编译

```bash
# 需要 Rust + Python + maturin
cd libriichi
maturin build --release
# 产物在 target/wheels/
```

## 预编译下载

从 [Releases](../../releases) 下载对应平台的预编译文件：
- `libriichi-*-x86_64-unknown-linux-gnu.so` (Linux x64)
- `libriichi-*-aarch64-apple-darwin.so` (macOS ARM)
- `libriichi-*-x86_64-pc-windows-msvc.pyd` (Windows x64)

## 验证

已通过 200 条天凤三麻凤凰桌牌谱（1832 局）的完整事件流验证，与原始数据 100% 一致。

## 许可证

AGPL-3.0，基于 [Equim-chan/Mortal](https://github.com/Equim-chan/Mortal)。

## 致谢

- [Mortal](https://github.com/Equim-chan/Mortal) — 原始四麻 AI 引擎
