/**
 * 合作方配置文件
 * 统一管理所有合作方信息和项目映射
 *
 * 添加新合作方：直接在 affiliates 中添加一个新条目即可
 */

export interface Affiliate {
  name: string;
  link: string;
  code?: string;
  /** 关联的项目ID列表 */
  projects: string[];
  /** 是否为组织级别的合作（显示不同的文案） */
  isOrganization?: boolean;
}

/**
 * 合作方配置
 * key: 合作方唯一标识 (用于 URL 参数 aff=xxx)
 */
export const affiliates: Record<string, Affiliate> = {
  wutuobang: {
    name: "乌托邦",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["1p2TFl6X"],
  },
  wuye: {
    name: "吴也MC",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: [
      "NxtrWNas", // 探索自然2
      "Z1Z1xI1K", // 自然之旅3
      "Gd9LgTCW", // 悠然人生1
      "TJTmchrm", // 悠然人生2
      "w71BhsmT", // 灾难降临
      "yHBuGZk1", // 悠然人生3
      "tFpySPqY", // 自然之旅1
      "pC0EfVWW", // 探索自然1
    ],
  },
  cuiguzheng: {
    name: "脆骨症",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: [
      "fZSAKVSg", // 脆骨症
      "dL0Tbr7N", // 脆骨症：黯光
      "DqpQXzc2", // 脆骨症2
    ],
  },
  grannixie: {
    name: "浙水院Minecraft社",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["r0WJ4XSq"], // 群峦：重生
  },
  Unknown_Entity_: {
    name: "Unknown_Entity_",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["tRR4pnOA"], // 机械动力，无限构件
  },
  ruoling: {
    name: "龙之冒险:新征程",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["EIrkPpcm"],
  },
  JQKA326: {
    name: "香草纪元:食旅纪行",
    code: "香草纪元",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["e11vzqXl"],
  },
  song_5007: {
    name: "song_5007",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["S5mhiSMC"],
  },
  Puikre: {
    name: "Puikre",
    link: "https://item.taobao.com/item.htm?id=807034865363&sku_properties=122216883%3A27889",
    projects: [
      "CFqHhpsh", // 锻造大师
      "KGIfMlOP", // 赏金猎人
    ],
  },
  ZangHeRo: {
    name: "ZangHeRo",
    link: "https://item.taobao.com/item.htm?id=807034865363&sku_properties=122216883%3A27889",
    projects: ["F4xIzfIX", "k04waacm"], // 机械殖民地
  },
  snk: {
    name: "二十二度幻月",
    link: "https://item.taobao.com/item.htm?id=807034865363&sku_properties=122216883%3A27889",
    projects: ["G23dLUsP"], // 剑与王国
  },
  thefool: {
    name: "愚者：The Fool",
    code: "愚者",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["XMUypeti"],
  },
  skillet_man: {
    name: "平底锅侠",
    code: "平底锅侠",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["YgvldBV8"],
  },
  "ymzz": {
    name: "远梦之棺",
    code: "远梦之棺",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["g1Bj9aPx"],
  },
  "incandescent-verity": {
    name: "方可梦：炽白真形",
    code: "方可梦：炽白真形",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["qoClGU9p"],
  },
  Latxx: {
    name: "辣某人",
    code: "沉浸战斗",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["aa8zTitm"],
  },
  Karashok_Leo: {
    name: "咒次元",
    link: "https://item.taobao.com/item.htm?id=807034865363&sku_properties=122216883%3A27889",
    projects: ["k5OmCs1S"],
  },
  Lovin: {
    name: "勇者之章",
    link: "https://item.taobao.com/item.htm?id=807034865363&sku_properties=122216883%3A27889",
    projects: ["KgeSn4uG"],
  },
  JasonQ: {
    name: "齿轮盛宴",
    link: "https://item.taobao.com/item.htm?id=807034865363&sku_properties=122216883%3A27889",
    projects: ["OIIWCwpQ"],
  },
  Altnoir: {
    name: "空中厕所2",
    link: "https://item.taobao.com/item.htm?id=807034865363&sku_properties=122216883%3A27889",
    projects: ["FkZiwq64"],
  },
  wuwei: {
    name: "无畏灬大魔王",
    code: "真实地球",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["UJBwwyq3"],
  },
  ft_wt: {
    name: "农场物语",
    code: "农场物语",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["YtS91hhr"],
  },
  martyredroad: {
    name: "殉道之路",
    code: "殉道之路",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["2cDBzlDs"],
  },
  tfg: {
    name: "锻造之旅",
    code: "锻造之旅",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["92pKuCHs"],
  },
  shenhuaqiyuan: {
    name: "神话：起源4.7重制版",
    code: "神话起源",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["ZSSC3pSh"],
  },
  "unfinished-path": {
    name: "墨言",
    code: "墨言",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["zT3k10EZ", "i2V4lWdp", "37yTxfMk"],
  },
  "syqj": {
    name: "摄影奇境",
    code: "摄影奇境",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["iz5Q6p0G"],
  },
  deceasedcraft: {
    name: "亡者世界",
    code: "亡者世界",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["PRsqHZRu"],
  },
  LaotouY: {
    name: "BBSMC",
    code: "BBSMC",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: [], // 通过组织级别配置，不需要手动添加项目
    isOrganization: true,
  },
  bff: {
    name: "逆转未来",
    code: "逆转未来",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["vxAmfb3Y"],
  },
  cfgj: {
    name: "尘封古纪",
    code: "尘封古纪",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["PNjet8tS"],
  },
  heavenlypath: {
    name: "通天之路",
    code: "通天之路",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["Nt9lp3GH"],
  },
  arcadiaapocalypse: {
    name: "阿卡迪亚的天启",
    code: "阿卡迪亚的天启",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["ZvIENE9v"],
  },
  woyaozuofan: {
    name: "我要做饭",
    code: "我要做饭",
    link: "https://item.taobao.com/item.htm?ft=t&id=1052116079164",
    projects: ["LLxkeMHx"],
  },
};

// ============ 自动生成的映射表 (不要手动编辑) ============

/** 项目ID -> 合作方key 的映射 */
export const projectAffiliates: Record<string, string> = Object.fromEntries(
  Object.entries(affiliates).flatMap(([key, aff]) => aff.projects.map((pid) => [pid, key])),
);

/** 合作方详细信息 (兼容旧接口) */
export const creators: Record<
  string,
  { name: string; link: string; code?: string; isOrganization?: boolean }
> = Object.fromEntries(
  Object.entries(affiliates).map(([key, { name, link, code, isOrganization }]) => [
    key,
    { name, link, code, isOrganization },
  ]),
);

// ============ 辅助函数 ============

/**
 * 根据项目ID获取合作方信息
 */
export function getCreatorByProjectId(
  projectId: string,
): { name: string; link: string; code?: string; isOrganization?: boolean } | null {
  const affKey = projectAffiliates[projectId];
  return affKey ? (creators[affKey] ?? null) : null;
}

/**
 * 根据合作方key获取合作方信息
 */
export function getCreatorByKey(
  key: string,
): { name: string; link: string; code?: string; isOrganization?: boolean } | null {
  return creators[key] ?? null;
}

/**
 * 获取项目的合作方key
 */
export function getAffiliateKey(projectId: string): string | null {
  return projectAffiliates[projectId] ?? null;
}

/**
 * 检查项目是否有合作方
 */
export function hasAffiliate(projectId: string): boolean {
  return projectId in projectAffiliates;
}
