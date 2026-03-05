<template>
  <div :style="themeVars">
    <div v-if="cf" class="game-page container">
      <!-- 居中样式 -->
      <div style="justify-content: center; align-items: center">
        <h1 style="font-size: 30px; font-weight: bold; color: var(--color-text-dark)">
          整合包联机面板快速部署
        </h1>
        <h2
          data-v-56edd70f=""
          class="relative m-0 text-base font-normal leading-[155%] text-secondary md:text-[18px]"
        >
          已支持100+ <span data-v-56edd70f="" class="font-bold"> 主流整合包</span> ，
          快速一键部署，无需繁琐上传和配置即可联机。
          <br />
          <br />
          <template v-if="cf.isOrganization">
            您的购买带来的收益，将为
            <span data-v-56edd70f="" class="font-bold">{{ cf.name }}</span>
            提供持续的研发和运营支持
          </template>
          <template v-else>
            与
            <span data-v-56edd70f="" class="font-bold"> {{ cf.name }}</span>
            联动合作，您的购买将会把销售收益的20%-30%归于{{ cf.name }}所有用于创作持续性收益
          </template>
        </h2>
        <h1
          style="
            font-size: 30px;
            font-weight: bold;
            color: var(--color-text-dark);
            margin-top: 30px;
          "
        >
          咨询客服
        </h1>
        <h2
          data-v-56edd70f=""
          class="relative m-0 text-base font-normal leading-[155%] text-secondary md:text-[18px]"
        >
          下单跳转淘宝 <span data-v-56edd70f="" class="font-bold"> 咨询客服</span>
          ，在下单前，我们强烈建议先咨询淘宝店铺客服提供所想要游玩的整合包的在线人数，我们会根据在线人数推荐合适的套餐，
          <br />
        </h2>
        <h1
          style="
            font-size: 30px;
            font-weight: bold;
            color: var(--color-text-dark);
            margin-top: 30px;
          "
        >
          选择合适您的套餐
        </h1>
        <p class="text-[15px]">所标注人数为推荐同时在线范围，请根据实际情况选择合适的套餐</p>
        <p class="text-[15px]">游戏人数在套餐推荐人数之间请参照购买预算选择套餐</p>
        <p class="text-[15px]">
          例如：我想玩乌托邦，我有五个人玩，我预算足够我购买发烧型4核10G，我预算不够我购买EPYC型4核8G
        </p>
        <h1
          style="
            font-size: 30px;
            font-weight: bold;
            color: var(--color-text-dark);
            margin-top: 30px;
          "
        >
          优惠赠送
        </h1>
        <p v-if="cf.code" class="text-[15px]">
          使用作者专属优惠码: <strong>{{ cf.code }}</strong>
        </p>
        <p class="text-[15px]">
          {{ cf.code ? "获得" : "" }}下单赠送7天时长,带游戏内3张风景图好评再送7天
        </p>

        <!-- 套餐表格 -->
        <div class="pricing-table-wrapper">
          <table class="pricing-table">
            <thead>
              <tr>
                <th>套餐类型</th>
                <th>配置</th>
                <th>推荐人数</th>
                <th>价格</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              <!-- EPYC型 -->
              <tr class="tier-header epyc">
                <td colspan="5">
                  <div class="tier-info">
                    <span class="tier-name">EPYC型</span>
                    <span class="tier-specs"
                      >AMD 霄龙 EPYC 7R13（CPUZ 500分）| 小海豚PBlaze6 6537 企业级U.2固态 | RECC
                      DDR4 2400MHz 自动纠错</span
                    >
                  </div>
                </td>
              </tr>
              <tr v-for="plan in epycPlans" :key="plan.name" class="plan-row">
                <td class="plan-type">
                  <span class="plan-badge epyc">EPYC型</span>
                </td>
                <td class="plan-config">
                  <span class="config-highlight">{{ plan.cpu }}核 {{ plan.memory }}G</span>
                </td>
                <td class="plan-players">{{ plan.players }}</td>
                <td class="plan-price">
                  <span class="price">¥{{ plan.price }}</span>
                  <span class="price-unit">/月</span>
                </td>
                <td class="plan-action">
                  <div class="action-buttons">
                    <ButtonStyled color="green" type="outlined">
                      <nuxt-link :to="cf.link" target="_blank">电脑淘宝购买</nuxt-link>
                    </ButtonStyled>
                    <button class="mobile-taobao-btn" @click="showMobileQR = true">
                      手机淘宝购买
                    </button>
                  </div>
                </td>
              </tr>

              <!-- 发烧型 -->
              <tr class="tier-header enthusiast">
                <td colspan="5">
                  <div class="tier-info">
                    <span class="tier-name">发烧型</span>
                    <span class="tier-specs"
                      >Intel 酷睿 I7-14700K（CPUZ 870分）| 英特尔P4510 企业级U.2固态 | DDR4
                      3200MHz</span
                    >
                  </div>
                </td>
              </tr>
              <tr v-for="plan in enthusiastPlans" :key="plan.name" class="plan-row">
                <td class="plan-type">
                  <span class="plan-badge enthusiast">发烧型</span>
                </td>
                <td class="plan-config">
                  <span class="config-highlight">{{ plan.cpu }}核 {{ plan.memory }}G</span>
                </td>
                <td class="plan-players">{{ plan.players }}</td>
                <td class="plan-price">
                  <span class="price">¥{{ plan.price }}</span>
                  <span class="price-unit">/月</span>
                </td>
                <td class="plan-action">
                  <div class="action-buttons">
                    <ButtonStyled color="green" type="outlined">
                      <nuxt-link :to="cf.link" target="_blank">电脑淘宝购买</nuxt-link>
                    </ButtonStyled>
                    <button class="mobile-taobao-btn" @click="showMobileQR = true">
                      手机淘宝购买
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <!-- 手机淘宝二维码弹窗 -->
        <Teleport to="body">
          <div v-if="showMobileQR" class="qr-modal-overlay" @click="showMobileQR = false">
            <div class="qr-modal" @click.stop>
              <button class="qr-modal-close" @click="showMobileQR = false">
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="24"
                  height="24"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                >
                  <line x1="18" y1="6" x2="6" y2="18"></line>
                  <line x1="6" y1="6" x2="18" y2="18"></line>
                </svg>
              </button>
              <h3 class="qr-modal-title">手机淘宝扫码购买</h3>
              <img
                src="https://cdn.bbsmc.net/bbsmc/data/QioTNNGG/images/962f1e591f1a8058b28177c11d5ee0c76ecdd01f.png"
                alt="手机淘宝二维码"
                class="qr-code-image"
              />
              <p class="qr-modal-hint">打开手机淘宝APP扫描二维码</p>
            </div>
          </div>
        </Teleport>

        <!-- 通用说明 -->
        <div class="common-specs">
          <div class="spec-item">
            <span class="spec-icon">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 24 24"
                fill="currentColor"
                width="20"
                height="20"
              >
                <path
                  d="M4 5h16a2 2 0 012 2v10a2 2 0 01-2 2H4a2 2 0 01-2-2V7a2 2 0 012-2zm0 2v10h16V7H4zm2 2h2v2H6V9zm4 0h8v2h-8V9zm-4 4h2v2H6v-2zm4 0h8v2h-8v-2z"
                />
              </svg>
            </span>
            <span class="spec-text">默认30G存储磁盘，可免费扩展至50G</span>
          </div>
          <div class="spec-item">
            <span class="spec-icon">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 24 24"
                fill="currentColor"
                width="20"
                height="20"
              >
                <path
                  d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z"
                />
              </svg>
            </span>
            <span class="spec-text">机柜共享带宽 2500Mbps</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ButtonStyled } from "@modrinth/ui";
import { isDarkTheme } from "~/plugins/theme/themes.ts";
import { getCreatorByKey } from "~/config/affiliates.ts";

const router = useRouter();
const route = useRoute();

// 获取当前主题并设置CSS变量
const { $theme } = useNuxtApp();

// 设置主题相关CSS变量
const themeVars = computed(() => {
  if (isDarkTheme($theme?.active)) {
    return {
      "--carousel-gradient-end": "rgba(0, 0, 0, 0.8)",
    };
  } else {
    return {
      "--carousel-gradient-end": "rgba(255, 255, 255, 0.9)",
    };
  }
});

// 获取 aff 参数
const aff = route.query.aff;

// EPYC型套餐
const epycPlans = [
  {
    name: "epyc-4c8g",
    cpu: 4,
    memory: 8,
    players: "2-3人",
    price: 58,
  },
];

// 发烧型套餐
const enthusiastPlans = [
  {
    name: "enthusiast-4c10g",
    cpu: 4,
    memory: 10,
    players: "4-5人",
    price: 88,
  },
  {
    name: "enthusiast-8c12g",
    cpu: 8,
    memory: 12,
    players: "6-8人",
    price: 108,
  },
  {
    name: "enthusiast-8c18g",
    cpu: 8,
    memory: 18,
    players: "8-10人",
    price: 168,
  },
];

const cf = ref(getCreatorByKey(aff));

// 手机淘宝二维码弹窗状态
const showMobileQR = ref(false);

onMounted(() => {
  if (cf.value == null) {
    router.push("/");
  }
});
</script>

<style scoped>
.pricing-table-wrapper {
  margin-top: 24px;
  overflow-x: auto;
  border-radius: 12px;
  border: 1px solid var(--color-divider);
  background: var(--color-raised-bg);
}

.pricing-table {
  width: 100%;
  border-collapse: collapse;
  min-width: 600px;
}

.pricing-table th {
  background: var(--color-button-bg);
  padding: 16px 20px;
  text-align: left;
  font-weight: 600;
  color: var(--color-text-dark);
  border-bottom: 2px solid var(--color-divider);
  white-space: nowrap;
}

.pricing-table th:first-child {
  border-radius: 11px 0 0 0;
}

.pricing-table th:last-child {
  border-radius: 0 11px 0 0;
  text-align: center;
}

.tier-header {
  background: var(--color-button-bg);
}

.tier-header.epyc {
  background: linear-gradient(90deg, rgba(241, 100, 54, 0.1) 0%, transparent 100%);
}

.tier-header.enthusiast {
  background: linear-gradient(90deg, rgba(59, 130, 246, 0.1) 0%, transparent 100%);
}

.tier-header td {
  padding: 12px 20px;
  border-bottom: 1px solid var(--color-divider);
}

.tier-info {
  display: flex;
  align-items: center;
  gap: 16px;
  flex-wrap: wrap;
}

.tier-name {
  font-weight: 700;
  font-size: 16px;
  color: var(--color-text-dark);
  padding: 4px 12px;
  border-radius: 6px;
  background: var(--color-button-bg);
}

.tier-header.epyc .tier-name {
  background: rgba(241, 100, 54, 0.15);
  color: var(--flame, #f16436);
}

.tier-header.enthusiast .tier-name {
  background: rgba(59, 130, 246, 0.15);
  color: #3b82f6;
}

.tier-specs {
  font-size: 13px;
  color: var(--color-secondary);
}

.plan-row {
  transition: background-color 0.2s ease;
}

.plan-row:hover {
  background: var(--color-button-bg);
}

.plan-row td {
  padding: 16px 20px;
  border-bottom: 1px solid var(--color-divider);
  vertical-align: middle;
}

.plan-row:last-child td {
  border-bottom: none;
}

.plan-badge {
  display: inline-block;
  padding: 4px 10px;
  border-radius: 4px;
  font-size: 12px;
  font-weight: 600;
}

.plan-badge.epyc {
  background: rgba(241, 100, 54, 0.12);
  color: var(--flame, #f16436);
}

.plan-badge.enthusiast {
  background: rgba(59, 130, 246, 0.12);
  color: #3b82f6;
}

.plan-config {
  font-weight: 500;
}

.config-highlight {
  display: inline-block;
  padding: 6px 12px;
  background: var(--color-button-bg);
  border-radius: 6px;
  font-weight: 600;
  color: var(--color-text-dark);
  font-size: 14px;
}

.plan-players {
  color: var(--color-secondary);
  font-size: 14px;
}

.plan-price {
  white-space: nowrap;
}

.plan-price .price {
  font-size: 20px;
  font-weight: 700;
  color: var(--flame, #f16436);
}

.plan-price .price-unit {
  font-size: 13px;
  color: var(--color-secondary);
  margin-left: 2px;
}

.plan-action {
  text-align: center;
}

.plan-action a {
  text-decoration: none;
  color: inherit;
}

.action-buttons {
  display: flex;
  gap: 8px;
  justify-content: center;
  flex-wrap: wrap;
}

/* 手机淘宝购买按钮样式 */
.mobile-taobao-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 8px 16px;
  font-size: 14px;
  font-weight: 600;
  color: var(--flame, #f16436);
  background: transparent;
  border: 2px solid var(--flame, #f16436);
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s ease;
  white-space: nowrap;
}

.mobile-taobao-btn:hover {
  background: var(--flame, #f16436);
  color: white;
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(241, 100, 54, 0.3);
}

.mobile-taobao-btn:active {
  transform: translateY(0);
}

/* 二维码弹窗样式 */
.qr-modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
  animation: fadeIn 0.2s ease;
}

@keyframes fadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

.qr-modal {
  background: var(--color-raised-bg);
  border-radius: 16px;
  padding: 24px;
  max-width: 360px;
  width: 90%;
  text-align: center;
  position: relative;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
  animation: slideUp 0.3s ease;
}

@keyframes slideUp {
  from {
    opacity: 0;
    transform: translateY(20px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.qr-modal-close {
  position: absolute;
  top: 12px;
  right: 12px;
  background: var(--color-button-bg);
  border: none;
  border-radius: 8px;
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  color: var(--color-secondary);
  transition: all 0.2s ease;
}

.qr-modal-close:hover {
  background: var(--color-divider);
  color: var(--color-text);
}

.qr-modal-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--color-text-dark);
  margin: 0 0 16px 0;
}

.qr-code-image {
  max-width: 100%;
  height: auto;
  border-radius: 12px;
  border: 1px solid var(--color-divider);
}

.qr-modal-hint {
  margin: 16px 0 0 0;
  font-size: 14px;
  color: var(--color-secondary);
}

/* 通用说明样式 */
.common-specs {
  margin-top: 20px;
  display: flex;
  gap: 24px;
  flex-wrap: wrap;
  padding: 16px 20px;
  background: var(--color-raised-bg);
  border-radius: 8px;
  border: 1px solid var(--color-divider);
}

.spec-item {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--color-secondary);
  font-size: 14px;
}

.spec-icon {
  display: flex;
  align-items: center;
  color: var(--flame, #f16436);
}

.spec-text {
  color: var(--color-text);
}

/* 响应式 */
@media (max-width: 768px) {
  .tier-info {
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
  }

  .tier-specs {
    font-size: 12px;
  }

  .pricing-table th,
  .plan-row td {
    padding: 12px 12px;
  }

  .common-specs {
    flex-direction: column;
    gap: 12px;
  }
}

h1 {
  font-size: 18px;
  display: flex;
  justify-content: space-between;
  margin-bottom: 24px;
  color: var(--color-text-dark);
}

h2,
p {
  line-height: 1.45;
  color: var(--color-text);
}

.game-page {
  position: relative;
  z-index: 2;
  padding: 0 16px;
}

@media (max-width: 768px) {
  .game-page {
    padding: 0 20px;
  }
}

.container,
.element-container {
  max-width: 1224px;
  margin: auto;
}

.container {
  min-height: 724px;
}
</style>
