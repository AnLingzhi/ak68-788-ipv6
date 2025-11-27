# ndpfix

- 作用：在 OpenWrt/CPE 环境下为 LAN 侧设备自动添加 `/128` 主机路由并在 WAN 口开启 NDP 代理，修复 IPv6 Relay 下的回程黑洞。
- 核心原理：对每个终端的公网 IPv6 执行 `ip -6 route replace <ip>/128 dev <br-lan>` 与 `ip -6 neigh replace proxy <ip> dev <eth1>`，并开启 `net.ipv6.conf.*.proxy_ndp=1`。

## 编译
- 目标：在 Linux 上交叉编译静态二进制，推荐 `aarch64-unknown-linux-musl` 或 `armv7-unknown-linux-musleabihf`，视 CPE 架构而定。
- 方式：
  - 安装 Rust 与 `rustup`，添加目标：`rustup target add aarch64-unknown-linux-musl`
  - 使用 `cargo build --release --target aarch64-unknown-linux-musl`

## 部署
- 将 `target/<arch>/release/ndpfix` 拷贝到 CPE `/usr/bin/ndpfix` 并赋予执行权限。
- 以 `root` 运行，确保系统有 `ip` 与 `sysctl`。

## 使用
- 带参数模式（支持短/长参数）：
  - 单次修复：`ndpfix oneshot --wan eth1 --lan br-lan --ip 240e:xxxx:xxxx:xxxx::abcd`
  - 扫描批量：`ndpfix scan --wan eth1 --lan br-lan`
- 位置参数模式（在某些系统上更稳）：
  - 单次修复：`ndpfix oneshot eth1 br-lan 240e:xxxx:xxxx:xxxx::abcd`
  - 扫描批量：`ndpfix scan eth1 br-lan`
  - 可加详细输出：在带参数模式使用 `--verbose`

## 持久化
- 可在 OpenWrt 中将命令加入 `/etc/rc.local` 或热插拔脚本，定时调用 `scan`。
- 若终端 IPv6 动态变化，建议周期运行 `scan`。

## 注意
- 需以 `root` 执行。
- 若防火墙阻断转发，需放行 IPv6 FORWARD 或在测试阶段设置 `ip6tables -P FORWARD ACCEPT`。
