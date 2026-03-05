<template>
  <div>
    <section class="card">
      <h2 class="text-2xl">{{ formatMessage(messages.title) }}</h2>
      <p class="mb-4">您的个人资料信息可在 BBSMC 公开查看。</p>
      <label>
        <span class="label__title">{{ formatMessage(messages.profilePicture) }}</span>
        <span v-if="hasPendingAvatar" class="pending-badge">审核中</span>
      </label>
      <div class="avatar-changer">
        <Avatar
          :src="previewImage ? previewImage : pendingAvatarUrl || avatarUrl"
          size="md"
          circle
          :alt="auth.user.username"
        />
        <div class="input-stack">
          <FileInput
            :max-size="262144"
            :show-icon="true"
            class="btn"
            :prompt="formatMessage(commonMessages.uploadImageButton)"
            accept="image/png,image/jpeg,image/gif,image/webp"
            :disabled="hasPendingAvatar"
            @change="showPreviewImage"
          >
            <UploadIcon />
          </FileInput>
          <Button
            v-if="previewImage"
            :action="
              () => {
                icon = null;
                previewImage = null;
              }
            "
          >
            <UndoIcon />
            {{ formatMessage(messages.profilePictureReset) }}
          </Button>
          <Button v-if="hasPendingAvatar" color="danger" :action="() => cancelReview('avatar')">
            <XIcon />
            撤销审核
          </Button>
        </div>
      </div>
      <label for="username-field">
        <span class="label__title">{{ formatMessage(messages.usernameTitle) }}</span>
        <span v-if="hasPendingUsername" class="pending-badge">审核中</span>
        <span class="label__description">
          {{ formatMessage(messages.usernameDescription) }}
        </span>
      </label>
      <input id="username-field" v-model="username" type="text" :disabled="hasPendingUsername" />
      <Button
        v-if="hasPendingUsername"
        color="danger"
        :action="() => cancelReview('username')"
        class="cancel-review-btn"
      >
        <XIcon />
        撤销用户名审核
      </Button>
      <label for="bio-field">
        <span class="label__title">{{ formatMessage(messages.bioTitle) }}</span>
        <span v-if="hasPendingBio" class="pending-badge">审核中</span>
        <span class="label__description">
          {{ formatMessage(messages.bioDescription) }}
        </span>
      </label>
      <textarea id="bio-field" v-model="bio" type="text" :disabled="hasPendingBio" />
      <Button
        v-if="hasPendingBio"
        color="danger"
        :action="() => cancelReview('bio')"
        class="cancel-review-btn"
      >
        <XIcon />
        撤销简介审核
      </Button>
      <div v-if="hasUnsavedChanges" class="input-group">
        <Button color="primary" :action="() => saveChanges()">
          <SaveIcon /> {{ formatMessage(commonMessages.saveChangesButton) }}
        </Button>
        <Button :action="() => cancel()">
          <XIcon /> {{ formatMessage(commonMessages.cancelButton) }}
        </Button>
      </div>
      <div v-else class="input-group">
        <Button disabled color="primary" :action="() => saveChanges()">
          <SaveIcon />
          {{
            saved
              ? formatMessage(commonMessages.changesSavedLabel)
              : formatMessage(commonMessages.saveChangesButton)
          }}
        </Button>
        <Button :link="`/user/${auth.user.username}`">
          <UserIcon /> {{ formatMessage(commonMessages.visitYourProfile) }}
        </Button>
      </div>
    </section>
  </div>
</template>

<script setup>
import { UserIcon, SaveIcon, UploadIcon, UndoIcon, XIcon } from "@modrinth/assets";
import { Avatar, FileInput, Button } from "@modrinth/ui";
import { commonMessages } from "~/utils/common-messages.ts";

useHead({
  title: "个人资料设置 - BBSMC",
  meta: [{ name: "robots", content: "noindex, nofollow" }],
});

definePageMeta({
  middleware: "auth",
});

const { formatMessage } = useVIntl();

const messages = defineMessages({
  title: {
    id: "settings.profile.profile-info",
    defaultMessage: "个人资料",
  },
  description: {
    id: "settings.profile.description",
    defaultMessage:
      "您的个人资料信息可在 BBSMC 公开查看，也可通过 <docs-link>BBSMC API</docs-link> 访问。",
  },
  profilePicture: {
    id: "settings.profile.profile-picture.title",
    defaultMessage: "头像",
  },
  profilePictureReset: {
    id: "settings.profile.profile-picture.reset",
    defaultMessage: "重置",
  },
  usernameTitle: {
    id: "settings.profile.username.title",
    defaultMessage: "用户名",
  },
  usernameDescription: {
    id: "settings.profile.username.description",
    defaultMessage: "用于标识您个人资料的唯一名称（不区分大小写）。",
  },
  bioTitle: {
    id: "settings.profile.bio.title",
    defaultMessage: "简介",
  },
  bioDescription: {
    id: "settings.profile.bio.description",
    defaultMessage: "一段简短的自我介绍。",
  },
});

const auth = await useAuth();

const getPendingValue = (type) => {
  const review = (auth.value.user.pending_profile_reviews || []).find(
    (r) => r.review_type === type,
  );
  return review?.new_value;
};

const pendingUsernameVal = getPendingValue("username");
const pendingBioVal = getPendingValue("bio");
const username = ref(
  pendingUsernameVal !== undefined ? pendingUsernameVal : auth.value.user.username,
);
const bio = ref(pendingBioVal !== undefined ? pendingBioVal : auth.value.user.bio);
const avatarUrl = ref(auth.value.user.avatar_url);
const icon = shallowRef(null);
const previewImage = shallowRef(null);
const saved = ref(false);

// 当用户修改字段时重置 saved 状态
watch([username, bio, previewImage], () => {
  saved.value = false;
});

// 审核状态
const pendingReviews = computed(() => auth.value.user.pending_profile_reviews || []);
const pendingUsernameReview = computed(() =>
  pendingReviews.value.find((r) => r.review_type === "username"),
);
const pendingBioReview = computed(() => pendingReviews.value.find((r) => r.review_type === "bio"));
const pendingAvatarReview = computed(() =>
  pendingReviews.value.find((r) => r.review_type === "avatar"),
);
const hasPendingUsername = computed(() => !!pendingUsernameReview.value);
const hasPendingBio = computed(() => !!pendingBioReview.value);
const hasPendingAvatar = computed(() => !!pendingAvatarReview.value);
const pendingAvatarUrl = computed(() => {
  if (!pendingAvatarReview.value) return null;
  try {
    return JSON.parse(pendingAvatarReview.value.new_value).avatar_url;
  } catch {
    return null;
  }
});

function syncFromAuth() {
  const reviews = auth.value.user.pending_profile_reviews || [];
  const pendingUsername = reviews.find((r) => r.review_type === "username");
  const pendingBio = reviews.find((r) => r.review_type === "bio");
  username.value =
    pendingUsername?.new_value !== undefined ? pendingUsername.new_value : auth.value.user.username;
  bio.value = pendingBio?.new_value !== undefined ? pendingBio.new_value : auth.value.user.bio;
  avatarUrl.value = auth.value.user.avatar_url;
}

async function cancelReview(reviewType) {
  const review = pendingReviews.value.find((r) => r.review_type === reviewType);
  if (!review) return;
  startLoading();
  try {
    await useBaseFetch(`user/${auth.value.user.id}/profile_reviews/${review.id}/cancel`, {
      method: "POST",
    });
    await useAuth(auth.value.token);
    // 撤销后同步到真实值（审核数据已清除）
    syncFromAuth();
    addNotification({
      group: "main",
      title: "已撤销",
      text: "审核已撤销，您可以重新提交修改",
      type: "success",
    });
  } catch (err) {
    addNotification({
      group: "main",
      title: "撤销失败",
      text: err?.data?.description || "操作失败",
      type: "error",
    });
  }
  stopLoading();
}

const hasUnsavedChanges = computed(() => {
  const pendingU = pendingUsernameReview.value?.new_value;
  const expectedUsername = pendingU !== undefined ? pendingU : auth.value.user.username;
  const pendingB = pendingBioReview.value?.new_value;
  const expectedBio = pendingB !== undefined ? pendingB : auth.value.user.bio;
  return username.value !== expectedUsername || bio.value !== expectedBio || previewImage.value;
});

function showPreviewImage(files) {
  const reader = new FileReader();
  icon.value = files[0];
  reader.readAsDataURL(icon.value);
  reader.onload = (event) => {
    previewImage.value = event.target.result;
  };
}

function cancel() {
  icon.value = null;
  previewImage.value = null;
  syncFromAuth();
}

async function saveChanges() {
  startLoading();
  try {
    const pendingMessages = [];

    if (icon.value) {
      const iconResult = await useBaseFetch(
        `user/${auth.value.user.id}/icon?ext=${
          icon.value.type.split("/")[icon.value.type.split("/").length - 1]
        }`,
        {
          method: "PATCH",
          body: icon.value,
        },
      );
      icon.value = null;
      previewImage.value = null;
      if (iconResult?.pending_review) {
        pendingMessages.push("头像");
      }
    }

    const body = {};

    if (auth.value.user.username !== username.value) {
      body.username = username.value;
    }

    if (auth.value.user.bio !== bio.value) {
      body.bio = bio.value;
    }

    if (Object.keys(body).length > 0) {
      const editResult = await useBaseFetch(`user/${auth.value.user.id}`, {
        method: "PATCH",
        body,
      });
      if (editResult?.pending_review && editResult?.fields) {
        const fieldNames = { username: "用户名", bio: "简介" };
        editResult.fields.forEach((f) => pendingMessages.push(fieldNames[f] || f));
      }
    }

    await useAuth(auth.value.token);
    // 同步 ref 值（审核中的字段显示新值）
    syncFromAuth();

    if (pendingMessages.length > 0) {
      addNotification({
        group: "main",
        title: "已提交审核",
        text: `您的${pendingMessages.join("、")}修改已提交管理员审核，请耐心等待`,
        type: "warn",
      });
    } else {
      saved.value = true;
    }
  } catch (err) {
    addNotification({
      group: "main",
      title: "发生错误",
      text: err?.data?.description || err?.data || err?.message || "未知错误，请稍后重试",
      type: "error",
    });
  }
  stopLoading();
}
</script>
<style lang="scss" scoped>
.avatar-changer {
  display: flex;
  gap: var(--gap-lg);
  margin-top: var(--gap-md);
}

textarea {
  height: 6rem;
  width: 40rem;
  margin-bottom: var(--gap-lg);
}

.pending-badge {
  display: inline-block;
  padding: 0.1rem 0.5rem;
  margin-left: 0.5rem;
  border-radius: var(--radius-md);
  background: rgba(245, 158, 11, 0.15);
  color: rgb(245, 158, 11);
  font-size: 0.75rem;
  font-weight: 600;
}

.cancel-review-btn {
  margin-bottom: var(--gap-md);
}
</style>
