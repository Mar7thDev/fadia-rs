# 100% 加载与入场后网络提示排查

排查日期：2026-09-13（本机时间）。目标为当前安装的客户端与本地 fadia-rs 服务端。

## 已确认原因

加载条到 100% 后，客户端仍会等待角色接管、初始化 RPC、默认外观和角色物品列表等条件。仅发送资源或把 ServerReadyFlag 提前设为 true，不能完成这些条件。

1. **接管存在循环等待。** 角色就绪标记等待 `ServerAcknowledgePossession`，但首次 `ClientRestart` 原来放在更晚的 `ServerRequestActorItems` 中。两端互相等待。
2. **控制器缺少 Pawn 复制。** 补齐 `PlayerController.Pawn`（句柄 18），并在角色通道打开后发送首次接管通知。
3. **异步资源加载导致首次通知过早。** 首次通知到达时角色可能尚未可用。现在从已登记的 `World.player_controller_map` 查找控制器，每秒重试 `ClientRetryClientRestart`，收到正确角色的接管确认后停止，并发送角色就绪标记。原有 `NetConnection.player_controller` 字段未被赋值，不能作为重试入口。
4. **缺少默认外观数据。** `FashionDyeData.FashionID=None` 会使客户端直接跳过外观加载；现在复制 `DefaultFashion`、染色编号 -1 和默认外观选项。对应句柄为 90、91、92。
5. **角色物品列表为空。** 场景中的 `EquippedPlayers` 并不替代 `InventoryComponent.CharacterItems`。客户端要求两者均初始化。现在发送 `Client_SetCharacterItems`（RPC 6），使用配置中的 `DefaultCharacterID`，并与 PlayerState 的角色 NetID 对齐。
6. **进入后缺少心跳回复。** 日志显示 `Server_Heartbeat`（PlayerState RPC 479）持续落入未处理分支。现在收到心跳后返回 `Client_HeartbeatResponse`（RPC 167）。这属于服务端协议处理缺失，不是缺少客户端资源包。
7. **底层数据包序号回绕失效。** 数据包使用 14 位序号，16383 后必须回到 0。原实现使用普通大小比较和未截断的自增值，回绕后 ACK 状态不再正常推进。此外，发送记录未保存实际确认的入站序号，批量 ACK 还从错误的队列方向移除记录。现在使用模 16384 的序号距离，按顺序消费已确认记录，并保留两个方向各自独立的序号空间。

已有的活动数据层与任务初始化修复继续保留：`ClientDataLayerRPC` 的消息包含继承的参数数组、读取索引和 Type=0x2B，任务信息通过对应原生 RPC 初始化。

## 客户端侧定位依据

- `AHTPlayerState` 主角色就绪检查：RVA `0x7D99DE0`。检查角色外观对象非空，以及背包的 CharacterItems 数组非空。
- 外观加载入口：RVA `0x7C829D0`。空 FashionID 直接返回；`DefaultFashion` 使用角色蓝图默认外观资产。
- `Client_HeartbeatResponse` 实现：RVA `0x90BC5C0`。清除累计未回复时间与网络异常状态。
- 上述地址仅适用于本次正在运行的客户端构建，不应直接套用到其他版本。

## 交付与验证记录

- Release 构建日志：`run-logs/build-packet-wrap.log`。
- 测试日志：`run-logs/test-final.log`。
- 最终登录与心跳日志：`run-logs/wrap-fixed-game.out.log`。
- 运行二进制：`run-logs/bin-current/fadia-game-server.exe`，与 `target/release/fadia-game-server.exe` 一致。
- SHA-256：`0F0E2EB09A41206BF7C38F27A3B619DDB3901366A0A5770F1C5DDA1B7CE37374`。

上述日志和运行文件属于本机验收记录，不随源码提交。

临时原生函数调用只用于逐项定位；最终验收使用重新启动的客户端，检查服务端自动完成全部入场步骤。

入场验收：

- 21:31 从登录页点击进入游戏，全新客户端自动完成接管、初始化、外观和背包数据加载，未使用临时原生函数调用或强行修改就绪标记。
- `nte-dumper/out/stable-login-state.json` 记录了接管角色一致，全部加载任务指针归零。
- 场景、角色、HUD 正常显示，空格输入触发角色跳跃动作。
- 此后发现单独补心跳回复仍不足以解决连接提示，继续修复了底层序号回绕及 ACK 推进。
- 最终版本重新启动后，自动完成角色接管、物品列表和初始化 RPC；本机 `nte-dumper/out/wrap-fixed-state.json` 记录全部加载任务指针归零，接管角色一致。
- 最终版本在 21:43:04 至 21:45:04 的有效采样中，心跳回复从 8 次增至 32 次，未处理心跳、累计遗漏心跳和网络异常标记均为 0。后续采样因运行时读取失败退出，未将它记录为完整三分钟通过。
- 用户随后明确确认 heartbeat 已无问题并要求停止测试；据此结束连接复测。

`cargo test -p fadia-engine -p fadia-game-server --release` 共 5 项测试通过：4 项网络回归测试和 1 项初始化序列化测试。网络测试覆盖零起始序号、跨界新旧包判定、批量 ACK 记录，以及通过真实头部编解码进行 32780 轮双向收发、跨越两次完整序号周期。Release 构建与 `git diff --check` 均通过。

本次范围为登录、进入场景和心跳连接。私服仍有其他未实现的玩法 RPC，不能据此认定全部玩法已实现。
