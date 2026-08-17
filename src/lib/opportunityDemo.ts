export type OpportunityDemo = {
  petLine: string;
  title: string;
  routeMapStatus: string;
  nextAction: string;
  plan: Array<{
    title: string;
    detail: string;
  }>;
};

export const opportunityDemo: OpportunityDemo = {
  petLine: "我找到一个需要多 agent 持续推进的赚钱项目。",
  title: "AI 资料包小店",
  routeMapStatus: "已生成 MAH 路线图",
  nextAction: "下一步：让 MAH 分配研究、制作、发布和复盘任务",
  plan: [
    {
      title: "找需求",
      detail: "研究最近有人愿意付费的资料包方向，选一个最小可卖主题。",
    },
    {
      title: "做产品",
      detail: "让写作、设计和 Codex 分别完成内容、封面、页面和下载包。",
    },
    {
      title: "跑反馈",
      detail: "发布到一个渠道，定时收集点击、收藏、咨询和购买信号。",
    },
  ],
};
