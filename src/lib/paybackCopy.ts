import { formatUsd } from "./money";

export type PaybackStatus =
  | "未设置"
  | "未启动"
  | "刚开始"
  | "在路上"
  | "加速"
  | "临门一脚"
  | "已回本"
  | "血赚"
  | "猛猛蹬";

export type PaybackCopy = {
  status: PaybackStatus;
  title: string;
  subtitle: string;
  percent: number;
  ratio: number;
};

function ratioLabel(ratio: number): string {
  return ratio.toFixed(1);
}

export function paybackCopy(apiValueUsd: number, monthlyCostUsd: number): PaybackCopy {
  if (monthlyCostUsd <= 0) {
    return {
      status: "未设置",
      title: "先看看值多少钱",
      subtitle: "设置月费后计算回本",
      percent: 0,
      ratio: 0,
    };
  }

  const ratio = Math.max(0, apiValueUsd / monthlyCostUsd);
  const percent = Math.round(ratio * 100);
  const remainingUsd = formatUsd(Math.max(0, monthlyCostUsd - apiValueUsd));
  const surplusUsd = formatUsd(Math.max(0, apiValueUsd - monthlyCostUsd));

  if (percent === 0) {
    return {
      status: "未启动",
      title: "还没开蹬",
      subtitle: "本周期还没有可计价用量",
      percent,
      ratio,
    };
  }
  if (percent < 30) {
    return {
      status: "刚开始",
      title: "轻轻蹬一下",
      subtitle: `已赚回 ${percent}%`,
      percent,
      ratio,
    };
  }
  if (percent < 60) {
    return {
      status: "在路上",
      title: "开始回本了",
      subtitle: `还差 ${remainingUsd} 回本`,
      percent,
      ratio,
    };
  }
  if (percent < 80) {
    return {
      status: "加速",
      title: "再蹬一会儿",
      subtitle: `已经回了 ${percent}%`,
      percent,
      ratio,
    };
  }
  if (percent < 100) {
    return {
      status: "临门一脚",
      title: "马上回本",
      subtitle: `还差 ${remainingUsd}`,
      percent,
      ratio,
    };
  }
  if (percent < 150) {
    return {
      status: "已回本",
      title: "已经回本",
      subtitle: `多赚了 ${surplusUsd}`,
      percent,
      ratio,
    };
  }
  if (percent < 200) {
    return {
      status: "血赚",
      title: "这月值了",
      subtitle: `相当于 ${ratioLabel(ratio)} 倍订阅`,
      percent,
      ratio,
    };
  }
  return {
    status: "猛猛蹬",
    title: "猛猛蹬，血赚",
    subtitle: `相当于 ${ratioLabel(ratio)} 倍订阅`,
    percent,
    ratio,
  };
}
