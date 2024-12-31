<template>
  <NuxtLayout>
    <div class="main">
      <div class="error">
        <BrandTextLogo v-if="error.statusCode === 404" />
        <h1 v-else>An error occurred!</h1>
        <p>{{ error.message }}</p>
        <div class="button-group">
          <nuxt-link to="/" class="iconified-button raised-button brand-button">
            前往主页
          </nuxt-link>
        </div>
      </div>
    </div>
  </NuxtLayout>
</template>

<script setup>
defineProps({
  error: {
    type: Object,
    default() {
      return {
        statusCode: 1000,
        message: "Unknown error",
      };
    },
  },
});
</script>

<style lang="scss" scoped>
.main {
  margin: var(--spacing-card-lg) auto;
  width: calc(100% - 4rem);

  @media screen and (min-width: 800px) {
    width: 600px;
  }
}

.error {
  h1 {
    margin-bottom: 0.25rem;
  }

  svg {
    fill: var(--color-text);
    color: var(--color-text);

    width: 100%;
    height: auto;
  }
}
</style>
