import { readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const siteRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
const repositoryRoot = resolve(siteRoot, "..");
const shouldWrite = process.argv.includes("--write");

const headingReplacements = [
  [/\bTypst document compilation\b/g, "Typst 文档编译"],
  [/\bAuto-review evaluation corpus\b/g, "Auto Review 评估语料库"],
  [/\bTypst third-party license texts\b/g, "Typst 第三方许可证文本"],
  [/\bWindows Sandbox 手工验收 Runbook\b/g, "Windows 沙箱手工验收手册"],
  [/\bProvider-independent model contract\b/g, "供应商无关的模型契约"],
  [/\bOwnership and end-to-end flow\b/g, "所有权与端到端流程"],
  [/\bEnd-to-end flow\b/g, "端到端流程"],
  [/\bSecurity and determinism\b/g, "安全性与确定性"],
  [/\bVersioning and security gates\b/g, "版本与安全门控"],
  [/\bCurrent limitations and extension points?\b/g, "当前限制与扩展点"],
  [/\bCurrent limitations \/ Extension points?\b/g, "当前限制与扩展点"],
  [/\bCurrent limitations \/ Potential\b/g, "当前限制与潜在方向"],
  [/\bCurrent implementation\b/g, "当前实现"],
  [/\bCurrent status\b/g, "当前状态"],
  [/\bCurrent surface\b/g, "当前接口面"],
  [/\bStaged evolution\b/g, "分阶段演进"],
  [/\bReview Checklist\b/g, "审查清单"],
  [/\bPublic contract map\b/g, "公共契约地图"],
  [/\bBoundary and public contract\b/g, "边界与公共契约"],
  [/\bpublic contract\b/gi, "公共契约"],
  [/\bPublic API\b/g, "公共接口"],
  [/\bPublic model\b/g, "公共模型"],
  [/\bRuntime ownership\b/g, "运行时所有权"],
  [/\bInternal ownership and call path\b/g, "内部所有权与调用路径"],
  [/\bInternal ownership and drift signals\b/g, "内部所有权与漂移信号"],
  [/\bTests and modification impact\b/g, "测试与修改影响"],
  [/\bTests and extension points\b/g, "测试与扩展点"],
  [/\bFailure semantics and integration obligations\b/g, "失败语义与集成义务"],
  [/\bPackage validation path\b/g, "包校验路径"],
  [/\bBuilt-in provider definition\b/g, "内置供应商定义"],
  [/\bSource boundary\b/g, "来源边界"],
  [/\bCrate boundary\b/g, "Crate 边界"],
  [/\bModules and key symbols\b/g, "模块与关键符号"],
  [/\bValidation and limits\b/g, "校验与限制"],
  [/\bIdentity, recovery and continuation\b/g, "身份、恢复与继续"],
  [/\bProgress, interaction and failure\b/g, "进度、交互与失败"],
  [/\bStreamable HTTP security and lifecycle\b/g, "Streamable HTTP 安全性与生命周期"],
  [/\bBackend implementation checklist\b/g, "后端实现清单"],
  [/\bExternal driver adapter\b/g, "外部驱动适配器"],
  [/\bDriver 与 output lifecycle\b/g, "驱动与输出生命周期"],
  [/\bTermination semantics\b/g, "终止语义"],
  [/\bSnapshot → UI mapping\b/g, "快照 → UI 映射"],
  [/\bKeyboard state machine\b/g, "键盘状态机"],
  [/\bTerminal lifecycle\b/g, "终端生命周期"],
  [/\bFonts and licensing\b/g, "字体与许可证"],
  [/\bRequest 与 execution\b/g, "请求与执行"],
  [/\bOwnership rule\b/g, "所有权规则"],
  [/\bStatus contract\b/g, "状态契约"],
  [/\bMutation contract\b/g, "变更契约"],
  [/\bPatch contract\b/g, "补丁契约"],
  [/\bIntegration boundary\b/g, "集成边界"],
  [/\bFormat baseline\b/g, "格式基线"],
  [/\bStable identity\b/g, "稳定身份"],
  [/\bCatalog snapshot\b/g, "目录快照"],
  [/\bContext layering\b/g, "上下文分层"],
  [/\bFile resolver\b/g, "文件解析器"],
  [/\bCurrent contract\b/g, "当前契约"],
  [/\bProposed durable contract\b/g, "计划中的持久化契约"],
  [/\bCurrent limitations\b/g, "当前限制"],
  [/\bNon-goals\b/g, "不包含的目标"],
  [/\bAcceptance tests\b/g, "验收测试"],
  [/\bLifecycle and state machine\b/g, "生命周期与状态机"],
  [/\bResource lifecycle\b/g, "资源生命周期"],
  [/\bCompatibility policy\b/g, "兼容策略"],
  [/\bMethod inventory\b/g, "方法清单"],
  [/\bRouting and ownership\b/g, "路由与所有权"],
  [/\bDeadline and cancellation\b/g, "截止时间与取消"],
  [/\bRequest fixture\b/g, "请求样例"],
  [/\bSuccess fixture\b/g, "成功样例"],
  [/\bError fixtures\b/g, "错误样例"],
  [/\bSource of truth\b/g, "权威来源"],
  [/\bTypst integration\b/g, "Typst 集成"],
  [/\bTypst document contract\b/g, "Typst 文档契约"],
  [/\bResource store\b/g, "资源存储"],
  [/\bError mapping\b/g, "错误映射"],
  [/\bUpdate broker\b/g, "更新代理"],
  [/\bReview task\b/g, "审查任务"],
  [/\bRecommendations\b/g, "建议"],
  [/\bAuto approval matrix\b/g, "自动批准矩阵"],
  [/\bCreate、Fork 与 Spawn\b/g, "创建（Create）、分叉（Fork）与生成（Spawn）"],
  [/\bResult 与 Join\b/g, "结果与汇合"],
  [/\bMain Process\b/g, "主进程"],
  [/\bIntegrated Terminal\b/g, "集成终端"],
  [/\bBrowser Capability\b/g, "浏览器能力"],
  [/\bCancel and disconnect\b/g, "取消与断开连接"],
  [/\bSession commands\b/g, "会话命令"],
  [/\bStable errors\b/g, "稳定错误"],
  [/\bSession model\b/g, "会话模型"],
  [/\bUpdate stream\b/g, "更新流"],
  [/\bSerialization contract\b/g, "序列化契约"],
  [/\bModel contract\b/g, "模型契约"],
  [/\bBundled runtime layout\b/g, "内置运行时布局"],
  [/\bInput、output 与 failure\b/g, "输入、输出与失败"],
  [/\bGeneration 与 failure semantics\b/g, "生成与失败语义"],
  [/\bFailure 与 shutdown\b/g, "失败与关闭"],
  [/\bProvider adapter pattern\b/g, "供应商适配器模式"],
  [/\bOpenAI service surface 不是 OpenAI-compatible profile\b/g, "OpenAI 服务接口面不是 OpenAI 兼容配置"],
  [/\bContent-addressed immutable store\b/g, "内容寻址的不可变存储"],
  [/\bAgent-facing helper tools\b/g, "面向 Agent 的辅助工具"],
  [/\bInstruction precedence\b/g, "指令优先级"],
  [/\bStructural invariants\b/g, "结构不变量"],
  [/\bDurable checkpoint\b/g, "持久化检查点"],
  [/\bRecovery and invalidation\b/g, "恢复与失效"],
  [/\bState machine\b/g, "状态机"],
  [/\bTyped command replay\b/g, "类型化命令重放"],
  [/\bbreaking change\b/g, "破坏性变更"],
  [/\bDurable sequence\b/g, "持久化序列"],
  [/\bTransient stream cursor\b/g, "临时流游标"],
  [/\bPresentation mapping\b/g, "展示映射"],
  [/\bEvaluation\b/g, "评估"],
  [/\bObservability\b/g, "可观测性"],
  [/\bAccessibility\b/g, "无障碍"],
  [/\bAttempt classifier\b/g, "尝试分类器"],
  [/\bSandbox Bridge\b/g, "沙箱桥接"],
  [/\bReview checklist\b/g, "审查清单"],
  [/\bPrecedence\b/g, "优先级"],
  [/\bSelection\b/g, "选择"],
  [/\bReferences\b/g, "参考资料"],
  [/\bScripts\b/g, "脚本"],
  [/\bAssets\b/g, "资源"],
  [/\bConnection lifecycle\b/g, "连接生命周期"],
  [/\bServer primitives\b/g, "服务端原语"],
  [/\bClient features\b/g, "客户端功能"],
  [/\bExperimental tasks\b/g, "实验性任务"],
  [/\bData egress\b/g, "数据外发"],
  [/\bDurability\b/g, "持久性"],
  [/\bFilter pipeline\b/g, "筛选流程"],
  [/\blist lookup\b/g, "列表查找"],
  [/\bMetadata\b/g, "元数据"],
  [/\bIn scope\b/g, "范围内"],
  [/\bOut of scope\b/g, "范围外"],
  [/\bState owner\b/g, "状态所有者"],
  [/\bPreconditions\b/g, "前置条件"],
  [/\bNotifications\b/g, "通知"],
  [/\bCompatibility\b/g, "兼容性"],
  [/\bIdempotency\b/g, "幂等性"],
  [/\bSemantics\b/g, "语义"],
  [/\bOrdering\b/g, "顺序"],
  [/\bSecurity\b/g, "安全性"],
  [/\bVerification\b/g, "验证"],
  [/\bConfiguration\b/g, "配置"],
  [/\bTelemetry\b/g, "遥测"],
  [/\bRendering\b/g, "渲染"],
  [/\bTransport\b/g, "传输"],
  [/\bInitialize\b/g, "初始化"],
  [/\bDecision\b/g, "决策"],
  [/\bOwnership\b/g, "所有权"],
  [/\bCurrent\b/g, "当前状态"],
  [/\bProposed\b/g, "计划"],
  [/\bPotential\b/g, "潜在方向"],
  [/\bScope\b/g, "范围"],
  [/\bParams\b/g, "参数"],
  [/\bResult\b/g, "结果"],
  [/\bErrors\b/g, "错误"],
  [/\bError\b/g, "错误"],
  [/\bRetry\b/g, "重试"],
  [/\bBackoff\b/g, "退避"],
  [/\bLiveness\b/g, "活性"],
  [/\bOutput\b/g, "输出"],
  [/\bPhase\b/g, "阶段"],
  [/\bsource\b/gi, "来源"],
  [/\bruntime\b/gi, "运行时"],
  [/\bidentity\b/gi, "身份"],
  [/\bbinding\b/gi, "绑定"],
  [/\bprovenance\b/gi, "来源"],
  [/\bdefinition\b/gi, "定义"],
  [/\bschema\b/gi, "模式"],
  [/\bspec\b/gi, "规格"],
  [/\bleaf\b/gi, "叶级"],
  [/\baggregate\b/gi, "聚合"],
  [/\bexposure\b/gi, "暴露范围"],
  [/\bregistry\b/gi, "注册表"],
  [/\bsnapshot\b/gi, "快照"],
  [/\binput\b/gi, "输入"],
  [/\bsafe point\b/gi, "安全点"],
  [/\bdrain\b/gi, "排空"],
  [/\bpayload\b/gi, "载荷"],
  [/\bmaterialized\b/gi, "具体化"],
  [/\binvocation\b/gi, "调用"],
  [/\bexecutor\b/gi, "执行器"],
  [/\bconsumer\b/gi, "消费方"],
  [/\bmodel-facing\b/gi, "面向模型的"],
  [/\bexecution\b/gi, "执行"],
  [/\boutcome\b/gi, "结果"],
  [/\badapters?\b/gi, "适配器"],
  [/\bconversion\b/gi, "转换"],
  [/\bdynamic\b/gi, "动态"],
  [/\bregistration\b/gi, "注册"],
  [/\bsearch\b/gi, "搜索"],
  [/\bindex\b/gi, "索引"],
  [/\branking\b/gi, "排序"],
  [/\bloading flow\b/gi, "加载流程"],
  [/\bhelper\b/gi, "辅助程序"],
  [/\bcode mode\b/gi, "代码模式"],
  [/\bprojection\b/gi, "投影"],
  [/\bnested call path\b/gi, "嵌套调用路径"],
  [/\bshape\b/gi, "结构"],
  [/\bnormalization\b/gi, "规范化"],
  [/\bimage safety\b/gi, "图像安全"],
  [/\bprovider\b/gi, "供应商"],
  [/\bpolicy\b/gi, "策略"],
  [/\bheadless\b/gi, "无界面"],
  [/\bserver request\b/gi, "服务端请求"],
  [/\bremote\b/gi, "远程"],
  [/\bflow\b/gi, "流程"],
  [/\bscheduling\b/gi, "调度"],
  [/\bplane\b/gi, "平面"],
  [/\bvertical slice\b/gi, "纵向切片"],
  [/\bworker\b/gi, "工作进程"],
  [/\benvironment\b/gi, "环境"],
  [/\bartwork\b/gi, "图稿"],
  [/\binheritance\b/gi, "继承"],
  [/\bseed\b/gi, "种子"],
  [/\bsaga\b/gi, "事务"],
  [/\bcommunication\b/gi, "通信"],
  [/\bmessage\b/gi, "消息"],
  [/\bdelivery\b/gi, "投递"],
  [/\bsteering\b/gi, "引导"],
  [/\bcancellation\b/gi, "取消"],
  [/\bterminal semantics\b/gi, "终态语义"],
  [/\bcontext\b/gi, "上下文"],
  [/\bisolation\b/gi, "隔离"],
  [/\bapproval\b/gi, "批准"],
  [/\bcapability\b/gi, "能力"],
  [/\brecovery\b/gi, "恢复"],
  [/\bmerge\b/gi, "合并"],
  [/\bmodel\b/gi, "模型"],
  [/\bcredential\b/gi, "凭据"],
  [/\bsubscription backend\b/gi, "订阅后端"],
  [/\bheader\b/gi, "标头"],
  [/\btarget\b/gi, "目标"],
  [/\bcatalog\b/gi, "目录"],
  [/\bpairing\b/gi, "配对"],
  [/\bstreaming\b/gi, "流式处理"],
  [/\bdecoder\b/gi, "解码器"],
  [/\bcommit ordering\b/gi, "提交顺序"],
  [/\btool\b/gi, "工具"],
  [/\binteraction\b/gi, "交互"],
  [/\bbackpressure\b/gi, "背压"],
  [/\bpackage\b/gi, "包"],
  [/\bmanifest\b/gi, "清单"],
  [/\blayout\b/gi, "布局"],
  [/\btrust\b/gi, "信任"],
  [/\bauthority\b/gi, "权威"],
  [/\bstate\b/gi, "状态"],
  [/\bpermission\b/gi, "权限"],
  [/\bcontribution\b/gi, "贡献"],
  [/\bactivation\b/gi, "激活"],
  [/\bupdate\b/gi, "更新"],
  [/\brollback\b/gi, "回滚"],
  [/\buninstall\b/gi, "卸载"],
  [/\bdiagnostics\b/gi, "诊断"],
  [/\bbudget\b/gi, "预算"],
  [/\bfixture\b/gi, "样例"],
  [/\bowner\b/gi, "所有者"],
  [/\bcontract\b/gi, "契约"],
  [/\bfailure\b/gi, "失败"],
  [/\bsemantics\b/gi, "语义"],
  [/\bintegration\b/gi, "集成"],
  [/\btests?\b/gi, "测试"],
  [/\binterface\b/gi, "接口"],
  [/\bport\b/gi, "端口"],
  [/\boutput\b/gi, "输出"],
  [/\bvalue\b/gi, "值"],
  [/\bagent-facing\b/gi, "面向 Agent 的"],
  [/\btools\b/gi, "工具"],
  [/\bdiscovery\b/gi, "发现"],
  [/\brun-once\b/gi, "单次运行"],
  [/\bautomation\b/gi, "自动化"],
  [/\bmode\b/gi, "模式"],
  [/\bpersistence\b/gi, "持久化"],
  [/\bjoin\b/gi, "汇合"],
  [/\bmetadata\b/gi, "元数据"],
  [/\bownership\b/gi, "所有权"],
  [/\bservice surface\b/gi, "服务接口面"],
  [/\boperation\b/gi, "操作"],
  [/\bboundary\b/gi, "边界"],
  [/\breview\b/gi, "审查"],
  [/\beval\b/gi, "评估"],
  [/\bwording\b/gi, "表述"],
  [/\brecommendation\b/gi, "建议"],
  [/\bevidence kind\b/gi, "证据类型"],
  [/\bscope\b/gi, "范围"],
  [/\bsurface\b/gi, "接口面"],
  [/\bprerequisite\b/gi, "前置条件"],
  [/\bdeployment\b/gi, "部署"],
  [/\bbridge\b/gi, "桥接"],
  [/\bpublic\b/gi, "公共"],
  [/\bprofile\b/gi, "配置档案"],
  [/\bdigest\b/gi, "摘要"],
  [/\bprotocol probe\b/gi, "协议探测"],
  [/\bpackaged\b/gi, "打包的"],
  [/\bhost loopback\b/gi, "宿主回环"],
  [/\benforcement denial\b/gi, "强制执行拒绝"],
  [/\bbackend obligation\b/gi, "后端义务"],
  [/\border\b/gi, "顺序"],
  [/\bcompaction\b/gi, "压缩"],
  [/\binvalidation\b/gi, "失效"],
  [/\bwindow\b/gi, "窗口"],
  [/\bchange\b/gi, "变更"],
  [/\brollout\b/gi, "发布"],
  [/\boverlay\b/gi, "浮层"],
  [/\brequest\b/gi, "请求"],
  [/\bresponse\b/gi, "响应"],
  [/\bendpoint\b/gi, "端点"],
  [/\bclient\b/gi, "客户端"],
  [/\blayers\b/gi, "分层"],
  [/\bwire\b/gi, "协议格式"],
  [/\bstore ports\b/gi, "存储端口"],
  [/\bstorage\b/gi, "存储"],
  [/\btransport\b/gi, "传输"],
  [/\bmemory\b/gi, "记忆"],
  [/\bresult\b/gi, "结果"],
  [/\binference probing\b/gi, "推理探测"],
  [/\brefresh trigger\b/gi, "刷新触发条件"],
  [/\bcache\b/gi, "缓存"],
  [/\bvalidator\b/gi, "校验器"],
  [/\bstable sort\b/gi, "稳定排序"],
  [/\bprotocol values\b/gi, "协议值"],
  [/\bcore\b/gi, "核心"],
  [/\bexport\b/gi, "导出"],
  [/\bhash\b/gi, "哈希"],
  [/\bdrift signal\b/gi, "漂移信号"],
  [/\borchestration\b/gi, "编排"],
  [/\bframing\b/gi, "分帧"],
  [/\band\b/gi, "与"],
  [/\bfeatures\b/gi, "功能"],
  [/\bresources\b/gi, "资源"],
  [/\bprompts\b/gi, "提示词"],
  [/\broots\b/gi, "根目录"],
  [/\bauthorization\b/gi, "授权"],
  [/\bdurability\b/gi, "持久性"],
  [/\bactivity\b/gi, "活动"],
  [/\bchanges\b/gi, "变更"],
  [/\bartifacts\b/gi, "产物"],
  [/\bbackoff\b/gi, "退避"],
  [/\bresolve\b/gi, "解析"],
  [/\brequests\b/gi, "请求"],
  [/\bresource\b/gi, "资源"],
  [/\bdependency\b/gi, "依赖"],
];

function walkReadmes(directory) {
  const results = [];
  for (const entry of readdirSync(directory)) {
    if (entry === "target" || entry === "node_modules" || entry.startsWith(".")) continue;
    const path = join(directory, entry);
    const stats = statSync(path);
    if (stats.isDirectory()) {
      results.push(...walkReadmes(path));
    } else if (entry === "README.md") {
      results.push(path);
    }
  }
  return results;
}

function sourceDocuments() {
  const systemDocs = readdirSync(join(repositoryRoot, "docs"))
    .filter((entry) => entry.endsWith(".md"))
    .map((entry) => join(repositoryRoot, "docs", entry));
  return [...systemDocs, ...walkReadmes(join(repositoryRoot, "zeta-rs"))];
}

const humanFirstSystemDocs = new Set([
  "docs/app-server-client.md",
  "docs/auto-review.md",
  "docs/codex-app-server.md",
  "docs/config.md",
  "docs/core-context.md",
  "docs/core-multi-agent.md",
  "docs/core.md",
  "docs/login.md",
  "docs/mcp.md",
  "docs/model-provider-config.md",
  "docs/model-provider.md",
  "docs/models-manager.md",
  "docs/permissions.md",
  "docs/plugins.md",
  "docs/protocol.md",
  "docs/sandboxing.md",
  "docs/secrets.md",
  "docs/skills.md",
  "docs/tools.md",
  "docs/tui.md",
  "docs/zeta-agent-runtime-architecture.md",
  "docs/zeta-app-server-api.md",
  "docs/zeta-cli-architecture.md",
  "docs/zeta-code-architecture-codex-style-v2.md",
  "docs/zeta-desktop-architecture.md",
  "docs/zeta-rs-architecture.md",
]);

function withoutInlineCode(line) {
  return line.replace(/`[^`]*`/g, "");
}

function normalizeHeading(line) {
  const match = /^(#{1,4}\s+)(.*)$/.exec(line);
  if (!match) return line;
  const segments = match[2].split(/(`[^`]*`)/g);
  const normalized = segments
    .map((segment, index) => {
      if (index % 2 === 1) return segment;
      return headingReplacements.reduce(
        (value, [pattern, replacement]) => value.replace(pattern, replacement),
        segment,
      );
    })
    .join("");
  const formatted = normalized
    .replace(/([\p{Script=Han}])\s+([\p{Script=Han}])/gu, "$1$2")
    .replace(/([\p{Script=Han}])([A-Za-z0-9`])/gu, "$1 $2")
    .replace(/([A-Za-z0-9`])([\p{Script=Han}])/gu, "$1 $2")
    .replace(/\s+([、：，。；])/gu, "$1")
    .replace(/([（【])\s+/gu, "$1")
    .replace(/\s+([）】])/gu, "$1")
    .replace(/^(\d+(?:\.\d+)*\.?)\s*/, "$1 ")
    .replace(/^阶段\s*([A-Z0-9][A-Z0-9.]*)/, "阶段 $1");
  return `${match[1]}${formatted}`;
}

function checkDocument(path) {
  const failures = [];
  const source = readFileSync(path, "utf8").replace(/\r\n/g, "\n");
  const lines = source.split("\n");
  let fenced = false;
  let topLevelHeadings = 0;
  let firstContentLine = null;
  let changed = false;

  for (let index = 0; index < lines.length; index += 1) {
    let line = lines[index];
    const trimmed = line.trim();
    if (!firstContentLine && trimmed) firstContentLine = { index, value: trimmed };
    if (/^```/.test(trimmed)) {
      fenced = !fenced;
      continue;
    }
    if (fenced) continue;
    const normalizedHeading = normalizeHeading(line);
    if (normalizedHeading !== line) {
      if (shouldWrite) {
        lines[index] = normalizedHeading;
        line = normalizedHeading;
        changed = true;
      } else {
        failures.push(`${index + 1}: 标题应写成“${normalizedHeading.replace(/^#{1,4}\s+/, "")}”`);
      }
    }
    if (/^#\s+/.test(line)) topLevelHeadings += 1;

    const prose = withoutInlineCode(line);
    if (/<br\s*\/?>/i.test(prose)) {
      failures.push(`${index + 1}: 使用了 HTML 换行；请改成段落、列表或表格`);
    }
    if (!line.startsWith(">") && /\s{2,}$/.test(line)) {
      failures.push(`${index + 1}: 使用了 Markdown 强制换行；请改成真实语义结构`);
    }
    const latinWordCount = prose.match(/[A-Za-z][A-Za-z-]*/g)?.length ?? 0;
    const isStructuralLine = /^(?:#{1,6}\s|[-*+]\s|\d+\.\s|\||>|<|\[|https?:\/\/)/.test(trimmed);
    if (!isStructuralLine && !/\p{Script=Han}/u.test(prose) && latinWordCount >= 8) {
      failures.push(`${index + 1}: 中文文档出现整行英文叙述；代码标识符保留英文，说明文字改用中文`);
    }
  }

  if (!firstContentLine || !/^#\s+/.test(firstContentLine.value)) {
    failures.push(`${(firstContentLine?.index ?? 0) + 1}: 文档必须以一级标题开始`);
  }
  if (topLevelHeadings !== 1) {
    failures.push(`一级标题数量应为 1，当前为 ${topLevelHeadings}`);
  }
  const sourcePath = relative(repositoryRoot, path).replaceAll("\\", "/");
  if (humanFirstSystemDocs.has(sourcePath) && !/^## 快速理解$/m.test(lines.join("\n"))) {
    failures.push("权威系统文档必须先提供“快速理解”章节，再进入内部实现");
  }
  if (changed) writeFileSync(path, lines.join("\n"));

  return failures;
}

const failures = sourceDocuments().flatMap((path) =>
  checkDocument(path).map((failure) => `${relative(repositoryRoot, path)}:${failure}`),
);

if (failures.length) {
  console.error("文档规范检查失败：");
  for (const failure of failures) console.error(`- ${failure}`);
  process.exitCode = 1;
} else {
  console.log(`${shouldWrite ? "Normalized" : "Checked"} ${sourceDocuments().length} documentation sources.`);
}
