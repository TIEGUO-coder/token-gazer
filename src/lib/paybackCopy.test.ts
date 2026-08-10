import { describe, expect, test } from "vitest";
import { paybackCopy } from "./paybackCopy";

describe("paybackCopy", () => {
  test("uses setup copy when monthly cost is not set", () => {
    expect(paybackCopy(12, 0)).toMatchObject({
      status: "未设置",
      title: "先看看值多少钱",
      subtitle: "设置月费后计算回本",
      percent: 0,
    });
  });

  test("uses not started copy when there is no billable value", () => {
    expect(paybackCopy(0, 20)).toMatchObject({
      status: "未启动",
      title: "还没开蹬",
      subtitle: "本周期还没有可计价用量",
      percent: 0,
    });
  });

  test("uses early copy below 30 percent", () => {
    expect(paybackCopy(5.8, 20)).toMatchObject({
      status: "刚开始",
      title: "轻轻蹬一下",
      subtitle: "已赚回 29%",
      percent: 29,
    });
  });

  test("uses almost there copy below full payback", () => {
    expect(paybackCopy(18, 20)).toMatchObject({
      status: "临门一脚",
      title: "马上回本",
      subtitle: "还差 $2.00",
      percent: 90,
    });
  });

  test("uses paid copy at full payback", () => {
    expect(paybackCopy(25, 20)).toMatchObject({
      status: "已回本",
      title: "已经回本",
      subtitle: "多赚了 $5.00",
      percent: 125,
    });
  });

  test("uses high multiple copy after 150 percent", () => {
    expect(paybackCopy(32, 20)).toMatchObject({
      status: "血赚",
      title: "这月值了",
      subtitle: "相当于 1.6 倍订阅",
      percent: 160,
    });
  });

  test("uses wild copy at 200 percent and above", () => {
    expect(paybackCopy(40, 20)).toMatchObject({
      status: "猛猛蹬",
      title: "猛猛蹬，血赚",
      subtitle: "相当于 2.0 倍订阅",
      percent: 200,
    });
  });
});
