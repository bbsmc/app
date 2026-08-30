<template>
  <main class="result-page">
    <section class="result-card">
      <div class="result-icon">
        <CheckCircleIcon aria-hidden="true" />
      </div>

      <p class="result-kicker">云账户</p>
      <h1>{{ resultContent.title }}</h1>
      <p class="result-desc">{{ resultContent.description }}</p>

      <div class="result-note">
        <InfoIcon aria-hidden="true" />
        <span>{{ resultContent.note }}</span>
      </div>
    </section>
  </main>
</template>

<script setup>
import CheckCircleIcon from "~/assets/images/utils/check-circle.svg?component";
import InfoIcon from "~/assets/images/utils/info.svg?component";

const route = useRoute();

const action = computed(() => {
  const raw = Array.isArray(route.query.action) ? route.query.action[0] : route.query.action;
  return raw === "release" ? "release" : raw === "sign" ? "sign" : "unknown";
});

const resultContent = computed(() => {
  if (action.value === "release") {
    return {
      title: "解约已完成",
      description: "您已完成云账户解约。可以关闭当前页面。",
      note: "回到原来的 BBSMC 收益页面后，点击「我已完成解约」同步状态。",
    };
  }

  if (action.value === "sign") {
    return {
      title: "实名签约已完成",
      description: "您已完成云账户实名签约。可以关闭当前页面。",
      note: "回到原来的 BBSMC 收益页面后，点击「我已完成签约」同步状态。",
    };
  }

  return {
    title: "流程已完成",
    description: "云账户 H5 流程已结束。可以关闭当前页面。",
    note: "回到原来的 BBSMC 收益页面后，手动同步最新状态。",
  };
});

useHead({
  title: computed(() => `${resultContent.value.title} - BBSMC`),
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});
</script>

<style scoped lang="scss">
.result-page {
  min-height: min(70vh, 48rem);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--gap-xl) var(--gap-md);
}

.result-card {
  width: min(100%, 34rem);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--gap-md);
  padding: clamp(var(--gap-lg), 4vw, var(--gap-xl));
  border-radius: var(--radius-lg);
  background: var(--color-raised-bg);
  box-shadow: var(--shadow-card);
  text-align: center;
}

.result-icon {
  width: 4.5rem;
  height: 4.5rem;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  background: rgba(34, 197, 94, 0.12);
  color: var(--color-green);

  svg {
    width: 2.75rem;
    height: 2.75rem;
  }
}

.result-kicker {
  margin: 0;
  color: var(--color-text);
  font-size: var(--font-size-sm);
  font-weight: 700;
}

h1 {
  margin: 0;
  color: var(--color-heading);
  font-size: clamp(2rem, 6vw, 3rem);
  line-height: 1;
}

.result-desc {
  margin: 0;
  max-width: 28rem;
  color: var(--color-text);
}

.result-note {
  display: flex;
  align-items: flex-start;
  gap: var(--gap-sm);
  width: 100%;
  padding: var(--gap-sm) var(--gap-md);
  border-radius: var(--radius-md);
  background: var(--color-bg);
  color: var(--color-text);
  text-align: left;
  font-size: var(--font-size-sm);

  svg {
    width: 1.15rem;
    min-width: 1.15rem;
    height: 1.15rem;
    margin-top: 0.1rem;
  }
}

@media screen and (max-width: 600px) {
  .result-page {
    min-height: 100dvh;
    padding: var(--gap-lg) var(--gap-md);
  }
}
</style>
