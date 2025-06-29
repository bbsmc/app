<template>
  <div :style="themeVars">
    <div v-if="cf && cf.items && cf.items.length > 0" class="game-page container">
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
          例如：我想玩乌托邦，我有五个人玩，我预算足够我购买高频88或者发烧108，我预算不够我购买高频58或发烧78
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
        <p class="text-[15px]">下单赠送7天时长,带游戏内3张风景图好评再送7天</p>

        <!-- 套餐列表 ，横着3个card -->
        <div class="resource-list flex flex-row gap-4">
          <div
            v-for="item in cf.items"
            :key="item.name"
            class="card min-h-200 flex w-full flex-col"
          >
            <h2>{{ item.name }}</h2>
            <p class="text-[15px]">{{ item.p1 }}</p>
            <p class="text-[15px]">{{ item.p2 }}</p>

            <div v-for="item_ in item.data" :key="item_.player">
              <p class="text-[13px]">推荐: {{ item_.player }}人</p>
              <p class="text-[13px]">配置: {{ item_.cpu }}核{{ item_.memory }}G</p>
              <p class="text-[13px]">价格: {{ item_.price }}元</p>
              <br />
            </div>
            <div class="mt-auto">
              <ButtonStyled color="green" type="outlined" style="margin-top: 10px">
                <nuxt-link :to="cf.link" target="_blank">淘宝咨询下单</nuxt-link>
              </ButtonStyled>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from "vue";
import { ButtonStyled } from "@modrinth/ui";
import { isDarkTheme } from "~/plugins/theme/themes";

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

const creaters = {
  pcl: {
    name: "PCL2",
    link: "https://item.taobao.com/item.htm?ft=t&id=881229604563",
    items: [
      {
        name: "旗舰套餐",
        p1: "Intel 铂金至强 Platinum 8151(CPUZ 480分)",
        p2: "英特尔U.2 P4510，ECC DDR4 2400MHz",
        data: [
          {
            player: "3-4",
            cpu: "4",
            memory: "8",
            price: "38",
          },
          {
            player: "5-7",
            cpu: "8",
            memory: "12",
            price: "68",
          },
        ],
      },
      {
        name: "高频套餐",
        p1: "AMD 霄龙 EPYC 74F3(CPUZ 580分)",
        p2: "英特尔U.2 P4510，ECC DDR4 2400MHz",
        data: [
          {
            player: "3-4",
            cpu: "4",
            memory: "8",
            price: "58",
          },
          {
            player: "5-7",
            cpu: "8",
            memory: "12",
            price: "88",
          },
          {
            player: "8-10",
            cpu: "8",
            memory: "18",
            price: "118",
          },
        ],
      },
      {
        name: "发烧套餐",
        p1: "Intel 酷睿 I7-13700K(CPUZ 830分)",
        p2: "英特尔U.2 P4510，DDR4 3200MHz",
        data: [
          {
            player: "3-4",
            cpu: "4",
            memory: "8",
            price: "78",
          },
          {
            player: "5-7",
            cpu: "8",
            memory: "12",
            price: "108",
          },
          {
            player: "8-10",
            cpu: "8",
            memory: "18",
            price: "168",
          },
        ],
      },
    ],
  },
};

const cf = ref(creaters.pcl);
</script>
<style scoped>
.resource-list {
  display: flex;
  flex-wrap: wrap;
  /* 允许换行 */
  gap: 24px;
  /* 默认间距 */
}

/* 媒体查询：当屏幕宽度小于 768px 时 */
@media (max-width: 768px) {
  .resource-list {
    flex-direction: column;
    /* 切换为单列布局 */
    gap: 0;
    /* 移动设备上不使用间距 */
  }
}

.card {
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  height: 100%;
  flex: 1 1 calc(33.333% - 24px);
  /* 三个卡片横向排列 */
  box-sizing: border-box;
  /* 包含边距和填充 */
}

/* 媒体查询：当屏幕宽度小于 768px 时 */
@media (max-width: 768px) {
  .card {
    flex: 1 1 100%;
    /* 每行一个卡片 */
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
  /* 添加左右间距 */
}

/* 媒体查询：当屏幕宽度小于 768px 时 */
@media (max-width: 768px) {
  .game-page {
    padding: 0 20px;
    /* 移动设备上减少左右间距 */
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

.section-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 24px;
}

/* 媒体查询：当屏幕宽度小于 768px 时 */
@media (max-width: 768px) {
  .section-title {
    padding: 0 24px;
    /* 添加左右间距 */
  }
}

/* 媒体查询：当屏幕宽度小于 480px 时 */
@media (max-width: 480px) {
  .section-title {
    padding: 0 8px;
    /* 更小的左右间距 */
  }
}
</style>
