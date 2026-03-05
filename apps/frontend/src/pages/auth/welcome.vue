<template>
  <div>
    <h1>{{ formatMessage(messages.welcomeLongTitle) }}</h1>

    <section class="auth-form">
      <p>
        {{ formatMessage(messages.welcomeDescription) }}
      </p>

      <Checkbox
        v-model="subscribe"
        class="subscribe-btn"
        :label="formatMessage(messages.subscribeCheckbox)"
        :description="formatMessage(messages.subscribeCheckbox)"
      />

      <button class="btn btn-primary continue-btn centered-btn" @click="continueSignUp">
        {{ formatMessage(commonMessages.continueButton) }} <RightArrowIcon />
      </button>

      <p>
        <IntlFormatted :message-id="messages.tosLabel">
          <template #terms-link="{ children }">
            <NuxtLink to="/legal2/terms" class="text-link">
              <component :is="() => children" />
            </NuxtLink>
          </template>
          <template #privacy-policy-link="{ children }">
            <NuxtLink to="/legal2/privacy" class="text-link">
              <component :is="() => children" />
            </NuxtLink>
          </template>
        </IntlFormatted>
      </p>
    </section>
  </div>
</template>
<script setup>
import { Checkbox } from "@modrinth/ui";
import { RightArrowIcon } from "@modrinth/assets";

const { formatMessage } = useVIntl();

const messages = defineMessages({
  subscribeCheckbox: {
    id: "auth.welcome.checkbox.subscribe",
    defaultMessage: "订阅 BBSMC 更新通知",
  },
  tosLabel: {
    id: "auth.welcome.label.tos",
    defaultMessage:
      "注册即表示您同意 BBSMC 的<terms-link>用户协议</terms-link>和<privacy-policy-link>隐私政策</privacy-policy-link>。",
  },
  welcomeDescription: {
    id: "auth.welcome.description",
    defaultMessage: "感谢您创建账号。您现在可以关注和创建项目、接收喜爱项目的更新通知等！",
  },
  welcomeLongTitle: {
    id: "auth.welcome.long-title",
    defaultMessage: "欢迎来到 BBSMC！",
  },
  welcomeTitle: {
    id: "auth.welcome.title",
    defaultMessage: "欢迎",
  },
});

useHead({
  title: () => `${formatMessage(messages.welcomeTitle)} - BBSMC`,
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});

const subscribe = ref(true);

async function continueSignUp() {
  const route = useRoute();

  await useAuth(route.query.authToken);
  await useUser();

  if (subscribe.value) {
    try {
      await useBaseFetch("auth/email/subscribe", {
        method: "POST",
      });
    } catch {
      /* empty */
    }
  }

  if (route.query.redirect) {
    await navigateTo(route.query.redirect);
  } else {
    await navigateTo("/dashboard");
  }
}
</script>
