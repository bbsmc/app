<template>
  <div>
    <section class="universal-card">
      <h2>更多链接</h2>
      <div class="adjacent-input">
        <label
          id="project-issue-tracker"
          title="A place for users to report bugs, issues, and concerns about your project."
        >
          <span class="label__title">问题提交</span>
          <span class="label__description"> 用户举报项目错误、问题和疑虑的地方。 </span>
        </label>
        <input
          id="project-issue-tracker"
          v-model="issuesUrl"
          type="url"
          placeholder="填入可访问链接"
          maxlength="2048"
          :disabled="!hasPermission"
        />
      </div>
      <div class="adjacent-input">
        <label
          id="project-source-code"
          title="A page/repository containing the source code for your project"
        >
          <span class="label__title">开源地址</span>
          <span class="label__description"> 包含项目源代码的页面/存储库 </span>
        </label>
        <input
          id="project-source-code"
          v-model="sourceUrl"
          type="url"
          maxlength="2048"
          placeholder="填入可访问链接"
          :disabled="!hasPermission"
        />
      </div>
      <div class="adjacent-input">
        <label
          id="project-wiki-page"
          title="A page containing information, documentation, and help for the project."
        >
          <span class="label__title">Wiki页面</span>
          <span class="label__description"> 包含项目信息、文档和帮助的页面。 </span>
        </label>
        <input
          id="project-wiki-page"
          v-model="wikiUrl"
          type="url"
          maxlength="2048"
          placeholder="填入可访问链接"
          :disabled="!hasPermission"
        />
      </div>
      <span class="label">
        <span class="label__title">其他链接</span>
        <span class="label__description"> 添加 爱发电 QQ频道 哔哩哔哩 KOOK等网址 </span>
      </span>

      <div
        v-for="(donationLink, index) in donationLinks"
        :key="`donation-link-${index}`"
        class="input-group donation-link-group"
      >
        <input
          v-model="donationLink.url"
          type="url"
          maxlength="2048"
          placeholder="填入可访问链接"
          :disabled="!hasPermission"
          @input="updateDonationLinks"
        />
        <DropdownSelect
          v-model="donationLink.id"
          name="Donation platform selector"
          :options="[
            'bilibili',
            'pd-qq',
            'afdian',
            'site',
            'kook',
            'modrinth',
            'spigotmc',
            'curseforge',
            'quark',
            'baidu',
            'other',
          ]"
          :display-name="webDisplayLabel"
          placeholder="选择平台"
          render-up
          class="platform-selector"
          @update:model-value="updateDonationLinks"
        />
      </div>
      <div class="button-group">
        <button
          type="button"
          class="iconified-button brand-button"
          :disabled="!hasChanges"
          @click="saveChanges()"
        >
          <SaveIcon />
          保存
        </button>
      </div>
    </section>
  </div>
</template>

<script setup>
import { DropdownSelect } from "@modrinth/ui";
import SaveIcon from "~/assets/images/utils/save.svg?component";

const tags = useTags();

const props = defineProps({
  project: {
    type: Object,
    default() {
      return {};
    },
  },
  currentMember: {
    type: Object,
    default() {
      return null;
    },
  },
  patchProject: {
    type: Function,
    default() {
      return () => {};
    },
  },
});

const webDisplayLabel = (x) => {
  switch (x) {
    case "other":
      return "其他";
    case "site":
      return "发布地址";

    case "modrinth":
      return "Modrinth地址";

    case "bilibili":
      return "哔哩哔哩";

    case "pd-qq":
      return "QQ频道";

    case "oopz":
      return "Oopz频道";

    case "kook":
      return "KOOK频道";

    case "afdian":
      return "爱发电";

    case "spigotmc":
      return "水龙头";

    case "curseforge":
      return "CurseForge地址";
    case "quark":
      return "夸克网盘";
    case "baidu":
      return "百度网盘";
    default:
      return x;
  }
};

const issuesUrl = ref(props.project.issues_url);
const sourceUrl = ref(props.project.source_url);
const wikiUrl = ref(props.project.wiki_url);
const discordUrl = ref(props.project.discord_url);

const rawDonationLinks = JSON.parse(JSON.stringify(props.project.donation_urls));
rawDonationLinks.push({
  id: null,
  platform: null,
  url: null,
});
const donationLinks = ref(rawDonationLinks);

const hasPermission = computed(() => {
  const EDIT_DETAILS = 1 << 2;
  return (props.currentMember.permissions & EDIT_DETAILS) === EDIT_DETAILS;
});

const patchData = computed(() => {
  const data = {};

  if (checkDifference(issuesUrl.value, props.project.issues_url)) {
    data.issues_url = issuesUrl.value === "" ? null : issuesUrl.value.trim();
  }
  if (checkDifference(sourceUrl.value, props.project.source_url)) {
    data.source_url = sourceUrl.value === "" ? null : sourceUrl.value.trim();
  }
  if (checkDifference(wikiUrl.value, props.project.wiki_url)) {
    data.wiki_url = wikiUrl.value === "" ? null : wikiUrl.value.trim();
  }
  if (checkDifference(discordUrl.value, props.project.discord_url)) {
    data.discord_url = discordUrl.value === "" ? null : discordUrl.value.trim();
  }

  const validDonationLinks = donationLinks.value.filter((link) => link.url && link.id);

  if (
    validDonationLinks !== props.project.donation_urls &&
    !(
      props.project.donation_urls &&
      props.project.donation_urls.length === 0 &&
      validDonationLinks.length === 0
    )
  ) {
    data.donation_urls = validDonationLinks;
  }

  if (data.donation_urls) {
    data.donation_urls.forEach((link) => {
      const platform = tags.value.donationPlatforms.find((platform) => platform.short === link.id);
      link.platform = platform.name;
    });
  }

  return data;
});

const hasChanges = computed(() => {
  return Object.keys(patchData.value).length > 0;
});

async function saveChanges() {
  if (patchData.value && (await props.patchProject(patchData.value))) {
    donationLinks.value = JSON.parse(JSON.stringify(props.project.donation_urls));
    donationLinks.value.push({
      id: null,
      platform: null,
      url: null,
    });
  }
}

function updateDonationLinks() {
  const links = donationLinks.value;
  links.forEach((link) => {
    if (link.url) {
      const url = link.url.toLowerCase();
      if (url.includes("afdian.com")) {
        link.id = "afdian";
      }
    }
  });
  if (!links.find((link) => !(link.url && link.id))) {
    links.push({
      id: null,
      platform: null,
      url: null,
    });
  }
  donationLinks.value = links;
}

function checkDifference(newLink, existingLink) {
  if (newLink === "" && existingLink !== null) {
    return true;
  }
  if (!newLink && !existingLink) {
    return false;
  }
  return newLink !== existingLink;
}
</script>
<style lang="scss" scoped>
.donation-link-group {
  input {
    flex-grow: 2;
    max-width: 26rem;
  }

  :deep(.animated-dropdown .selected) {
    height: 40px;
  }
}

.button-group {
  justify-content: flex-start;
}
</style>
