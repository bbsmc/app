<template>
  <div>
    <div class="game-header">
      <div class="hero-container">
        <img src="https://cdn.bbsmc.net/raw/top.jpeg" alt="header" />
        <div class="desktop-only"></div>
      </div>
    </div>

    <div v-if="cf && cf.items && cf.items.length > 0" class="game-page container">
      <div class="game-description">
        <div class="game-title">
          <h1 class="section-title">联机搭建</h1>
          <span class="num-projects">BBSMC & {{ cf.name }}</span>
        </div>
      </div>

      <!-- 居中样式 -->
      <div style="justify-content: center; align-items: center">
        <h1 style="font-size: 30px; font-weight: bold; color: #e5e5e5">整合包联机面板快速部署</h1>
        <h2
          data-v-56edd70f=""
          class="relative m-0 text-base font-normal leading-[155%] text-secondary md:text-[18px]"
        >
          已支持100+ <span data-v-56edd70f="" class="font-bold"> 主流整合包</span> ，
          快速一键部署，无需繁琐上传和配置即可联机。
          <br />
          <br />
          与
          <span data-v-56edd70f="" class="font-bold"> {{ cf.name }}</span>
          联动合作，您的购买将会把销售收益的20%-30%的归于{{ cf.name }}所有用于创作持续性收益
        </h2>
        <h1 style="font-size: 30px; font-weight: bold; color: #e5e5e5; margin-top: 30px">
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
        <h1 style="font-size: 30px; font-weight: bold; color: #e5e5e5; margin-top: 30px">
          选择合适您的套餐
        </h1>
        <p class="text-[15px]">所标注人数为推荐同时在线范围，请根据实际情况选择合适的套餐</p>
        <p class="text-[15px]">游戏人数在套餐推荐人数之间请参照购买预算选择套餐</p>
        <p class="text-[15px]">
          例如：我想玩乌托邦，我有五个人玩，我预算足够我购买高频88或者发烧108，我预算不够我购买高频58或发烧78
        </p>
        <h1 style="font-size: 30px; font-weight: bold; color: #e5e5e5; margin-top: 30px">
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
import { ref, onMounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ButtonStyled } from "@modrinth/ui";

const router = useRouter();
const route = useRoute();

// 获取 aff 参数
const aff = route.query.aff;

const creaters = {
  wuye: {
    name: "吴也MC",
    link: "https://item.taobao.com/item.htm?ft=t&id=874809176779",
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
  cuiguzheng: {
    name: "脆骨症",
    link: "https://item.taobao.com/item.htm?ft=t&id=791787996763",
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
  wutuobang: {
    name: "乌托邦",
    link: "https://item.taobao.com/item.htm?ft=t&id=876821726196",
    items: [
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

const cf = ref(creaters[aff]);

onMounted(() => {
  if (cf.value == null) {
    router.push("/");
  }
});
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
  color: #e5e5e5;
}

h2,
p {
  line-height: 1.45;
  color: #e5e5e5;
}

.hero-container {
  width: 100%;
  height: 144px;
  position: relative;
}

.game-header img {
  width: 100%;
  height: 144px;
}

.game-header .hero-container {
  height: 144px;
  z-index: 1;
}

.game-header .hero-container img {
  width: 100%;
  height: 144px;
  display: block;
}

body:has(.game-page) .game-header {
  margin-bottom: -110px;
  background-repeat: no-repeat;
}

body:has(.game-page) .game-header .hero-container:after {
  background: linear-gradient(hsla(0, 0%, 5%, 0.5), var(--color-background, #0d0d0d) 100%);
}

.game-header .hero-container:afterfont-weight {
  content: "";
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: -1px;
  background: linear-gradient(0deg, #0d0d0d, transparent);
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

.game-description h1 {
  line-height: 48px;
  font-family: var(--montserrat-font);
  font-weight: 700;
}

.game-description .game-title {
  margin-bottom: 12px;
  display: flex;
  align-items: center;
  white-space: nowrap;
}

.container,
.element-container {
  max-width: 1224px;
  margin: auto;
}

.container {
  min-height: 724px;
}

.game-description .num-projects {
  --space: 16px;
  position: relative;
  color: #e5e5e5;
  padding-left: var(--space);
  margin-left: var(--space);
  flex-shrink: 10;
  height: 48px;
  line-height: 48px;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.game-description .num-projects:before {
  position: absolute;
  margin-top: auto;
  margin-bottom: auto;
  top: 0;
  bottom: 0;
  left: 0;
  content: "";
  width: 1px;
  height: 28px;
  background: #4d4d4d;
}

.game-description .expandable-html-block {
  display: flex;
  gap: 8px;
}

.game-carousel {
  position: relative;
  width: 100%;
  height: 400px;
  overflow: hidden;
}

.carousel-items {
  position: relative;
  height: 100%;
  margin: 0;
  padding: 0;
  list-style: none;
}

.carousel-item {
  position: absolute;
  width: 80%;
  height: 100%;
  left: 50%;
  transition: all 0.5s ease;
  visibility: hidden;
}

.carousel-item.current {
  transform: translateX(-50%) scale(1);
  opacity: 1;
  z-index: 2;
  visibility: visible;
}

.carousel-item.previous {
  transform: translateX(-125%) scale(0.8);
  opacity: 0.6;
  z-index: 1;
  visibility: visible;
}

.carousel-item.next {
  transform: translateX(25%) scale(0.8);
  opacity: 0.6;
  z-index: 1;
  visibility: visible;
}

.carousel-image-container {
  width: 100%;
  height: 300px;
  overflow: hidden;
  border-radius: 8px;
}

.carousel-image-container img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.carousel-bottom-container {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  padding: 20px;
  background: linear-gradient(transparent, rgba(0, 0, 0, 0.8));
  color: #e5e5e5;
}

.carousel-item-title {
  font-size: 24px;
  font-weight: bold;
  margin-bottom: 10px;
}

.carousel-item-description {
  margin-bottom: 15px;
}

.carousel-buttons {
  position: absolute;
  top: 50%;
  left: 20px;
  right: 20px;
  transform: translateY(-50%);
  display: flex;
  justify-content: space-between;
  z-index: 3;
}

.carousel-buttons button {
  background: rgba(0, 0, 0, 0.5);
  border: none;
  color: #e5e5e5;
  width: 40px;
  height: 40px;
  border-radius: 50%;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
  transition: background-color 0.3s;
}

.carousel-buttons button:hover {
  background: rgba(0, 0, 0, 0.8);
}

.carousel-dots {
  position: absolute;
  bottom: 20px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  gap: 8px;
  z-index: 2;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.5);
  cursor: pointer;
}

.dot.active {
  background: #f1f1f1;
}

.modpacks-grid {
  display: grid;
  grid-template-columns: repeat(6, 1fr);
  gap: 16px;
  padding: 16px;
}

/* 媒体查询：当屏幕宽度小于 768px 时 */
@media (max-width: 768px) {
  .modpacks-grid {
    grid-template-columns: repeat(2, 1fr);
    /* 两列布局 */
  }
}

/* 媒体查询：当屏幕宽度小于 480px 时 */
@media (max-width: 480px) {
  .modpacks-grid {
    grid-template-columns: 1fr;
    /* 单列布局 */
  }
}

.modpack-card {
  background: rgb(16, 16, 19);
  border-radius: 8px;
  overflow: hidden;
  transition: transform 0.2s ease;
  display: flex;
  flex-direction: column;
}

.modpack-card:hover {
  transform: scale(1.05);
  z-index: 1;
}

.card-content {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100%;
}

.modpack-image {
  width: 100%;
  aspect-ratio: 1;
  object-fit: cover;
}

.modpack-basic-info {
  padding: 12px;
  margin-top: 12px;
  flex-grow: 1;
}

.modpack-info-top {
  flex-grow: 1;
}

.modpack-title {
  font-size: 16px;
  font-weight: 600;
  margin: 0 0 4px 0;
  color: #e5e5e5;
}

.modpack-author {
  font-size: 14px;
  color: #888;
  margin-bottom: 8px;
}

.modpack-footer {
  padding: 12px;
  border-top: 1px solid rgba(255, 255, 255, 0.1);
  background: rgb(16, 16, 19);
}

.modpack-stats {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 14px;
  color: #888;
}

.download-count {
  display: flex;
  align-items: center;
  gap: 4px;
}

.category {
  padding: 2px 8px;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 4px;
}

.modpack-link {
  text-decoration: none;
  color: inherit;
  display: block;
  height: 100%;
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

/* 媒体查询：当屏幕宽度小于 768px 时 */
@media (max-width: 768px) {
  .game-carousel {
    height: 300px;
    /* 手机端高度设置为 300px */
  }

  .carousel-image-container {
    height: 200px;
    /* 手机端高度设置为 200px */
  }
}
</style>
