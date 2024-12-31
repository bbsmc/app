<template>
  <ConfirmModal2
    ref="modalConfirmCreate"
    title="您正在申请 创建/修改 百科页面"
    description="请认真预览的内容:<br/><br/>1. 请确保您的内容符合社区规范<br/>2. 请确保您的内容不违反任何法律法规<br/>3. 请确保您的内容不侵犯他人的知识产权<br/>4. 请确保您的内容不包含任何敏感信息
    <br/><br/>
    5.提交后您将获得该资源5小时编辑权限,请在此期间内完成编辑<br/>6.提交后您的内容将会被审核,审核通过后将会公开展示在资源页面上<br/>7.审核不通过的内容将会被删除,请确保您的内容符合以上要求
    <br/>8.在获得5小时权限期间和提交审核期间,其他人不能重复申请编辑权限，<br/>若未在规定时间内完成编辑，将会被禁止申请编辑权限24小时<br/>
    <br/><br/>请认真阅读并确认您的内容符合以上要求后再提交"
    proceed-label="确认"
    @proceed="submitCreateWiki()"
  />

  <ConfirmModal2
    ref="modal_confirm_again"
    title="重新提交"
    description="您最多可以重复编辑5次，超过5次后将被禁止编辑3小时 "
    proceed-label="确认"
    @proceed="again()"
  />
  <ConfirmModal
    ref="modal_confirm_given_up"
    title="放弃提交"
    description="放弃提交后，您将被禁止编辑12小时，累计放弃和超时提交3次后将被禁止编辑3天"
    proceed-label="确认"
    @proceed="givenUp()"
  />

  <ConfirmModal2
    ref="modal_confirm_pass"
    title="确认审核通过"
    description="请确认您已经查看过该次提交是有效编辑，并且没有任何违法违规内容"
    proceed-label="确认"
    @proceed="accept()"
  />

  <NewModal ref="rejectWikiCache">
    <template #title>
      <Avatar :src="project.icon_url" :alt="project.title" class="icon" size="32px" />
      <div class="truncate text-lg font-extrabold text-contrast">百科审核</div>
    </template>

    <div class="flex flex-col gap-2">
      <label for="name">
        <span class="text-lg font-semibold text-contrast"> 拒绝理由: </span>
      </label>
      <textarea v-model="rejectWikiCacheMsg" type="text" placeholder="请输入拒绝该提交的理由" />
    </div>

    <div class="mt-5 flex gap-2" style="justify-content: flex-end">
      <ButtonStyled color="red">
        <button @click="confirmRejectWiki">
          <CheckIcon aria-hidden="true" />
          确认
        </button>
      </ButtonStyled>
    </div>
  </NewModal>

  <NewModal ref="viewBodyWikiView">
    <template #title>
      <button-styled
        @click="
          viewBodyWikiView.hide();
          preReviewWiki.show();
        "
      >
        <button>返回预览</button>
      </button-styled>
    </template>

    <div
      v-if="viewBodyWikiIsChange"
      style="
        width: 1600px;
        display: flex;
        justify-content: space-between;
        max-width: 100% !important;
      "
    >
      <div
        style="width: 49%"
        class="markdown-body"
        v-html="renderHighlightedString(viewBodyWiki.old_body)"
      />
      <div
        style="width: 49%"
        class="markdown-body"
        v-html="renderHighlightedString(viewBodyWiki.new_body)"
      />
    </div>
    <div v-else>
      <div
        style="width: 800px"
        class="markdown-body"
        v-html="renderHighlightedString(viewBodyWiki)"
      />
    </div>
  </NewModal>

  <NewModal ref="preReviewWiki">
    <template #title>
      <Avatar :src="project.icon_url" :alt="project.title" class="icon" size="32px" />
      <div class="truncate text-lg font-extrabold text-contrast">审核预览</div>
    </template>
    <ScrollablePanel
      :class="
        preIndexSetReview.length +
          preBodyReview.length +
          preADDReview.length +
          preSortReview.length +
          preREMOVEReview.length >
        3
          ? 'h-[30rem]'
          : ''
      "
    >
      <div class="flex flex-col gap-3" style="width: 800px">
        <div v-if="preIndexSetReview.length > 0" class="flex flex-col gap-2">
          <div class="flex flex-col gap-2">
            <label for="name">
              <span class="text-lg font-semibold text-contrast"> 设置主页: </span>
            </label>
            <div class="member">
              <div v-for="wiki_1 in preIndexSetReview" :key="wiki_1.id" class="member-header">
                <div class="info">
                  <div class="text">
                    <p>{{ wiki_1.title }}</p>
                  </div>
                </div>
                <div class="side-buttons">
                  <button-styled size="standard">
                    <button @click="viewBody(wiki_1.body)">查看新主页</button>
                  </button-styled>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div v-if="preBodyReview.length > 0" class="flex flex-col gap-2">
          <div class="flex flex-col gap-2">
            <label for="name">
              <span class="text-lg font-semibold text-contrast"> 修改正文: </span>
            </label>
            <div class="member">
              <div v-for="wiki_2 in preBodyReview" :key="wiki_2.id" class="member-header">
                <div class="info">
                  <div class="text">
                    <p>{{ wiki_2.title }}</p>
                  </div>
                </div>
                <div class="side-buttons">
                  <button-styled size="standard">
                    <button @click="viewBodyChange(wiki_2)">查看修改</button>
                  </button-styled>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div v-if="preSortReview.length > 0" class="flex flex-col gap-2">
          <div class="flex flex-col gap-2">
            <label for="name">
              <span class="text-lg font-semibold text-contrast"> 修改权重: </span>
            </label>
            <div class="member">
              <div v-for="wiki_3 in preSortReview" :key="wiki_3.id" class="member-header">
                <div class="info">
                  <div class="text">
                    <p>{{ wiki_3.title }}</p>
                  </div>
                </div>
                {{ wiki_3.old_sort_order }} -> {{ wiki_3.new_sort_order }}
              </div>
            </div>
          </div>
        </div>

        <div v-if="preADDReview.length > 0" class="flex flex-col gap-2">
          <div class="flex flex-col gap-2">
            <label for="name">
              <span class="text-lg font-semibold text-contrast"> 新增页面: </span>
            </label>
            <div class="member">
              <div
                v-for="wiki_new in preADDReview"
                :key="wiki_new.id"
                class="member-header"
                style="margin-top: 3px"
              >
                <div class="info">
                  <div class="text">
                    <p>{{ wiki_new.title }}</p>
                  </div>
                </div>
                <div class="side-buttons">
                  <button-styled size="standard">
                    <button @click="viewBody(wiki_new.body)">预览</button>
                  </button-styled>
                </div>
              </div>
            </div>
          </div>
        </div>
        <div v-if="preREMOVEReview.length > 0" class="flex flex-col gap-2">
          <div class="flex flex-col gap-2">
            <label for="name">
              <span class="text-lg font-semibold text-contrast"> 移除页面: </span>
            </label>
            <div class="member">
              <div
                v-for="wiki_remove_pre in preREMOVEReview"
                :key="wiki_remove_pre.id"
                class="member-header"
                style="margin-top: 3px"
              >
                <div class="info">
                  <div class="text">
                    <p>{{ wiki_remove_pre.title }}</p>
                  </div>
                </div>
                <div class="side-buttons">
                  <button-styled size="standard">
                    <button @click="viewBody(wiki_remove_pre.body)">预览</button>
                  </button-styled>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <span style="margin-top: 20px"></span>
    </ScrollablePanel>
    <div class="mt-5 flex gap-2">
      <ButtonStyled color="green">
        <button
          @click="
            () => {
              $refs.preReviewWiki.hide();
              $refs.modal_confirm_pass.show();
            }
          "
        >
          <CheckIcon aria-hidden="true" />
          通过
        </button>
      </ButtonStyled>
      <ButtonStyled color="red">
        <button style="margin-left: auto" @click="preRejectWikiCache">
          <XIcon aria-hidden="true" />
          拒绝
        </button>
      </ButtonStyled>
    </div>
  </NewModal>
  <!--  <span style="margin-left: 10px"></span>-->

  <section class="normal-page__content">
    <!--    页面编辑者-->
    <div
      v-if="
        props.wikis.is_editor && props.wikis.is_editor_user && props.wikis.cache.status === 'review'
      "
    >
      <div class="markdown-body card">
        <h2>百科页面正在审核中</h2>
        <br />

        <div class="member">
          <div class="member-header">
            <div class="info">
              <Avatar
                :src="props.wikis.editor_user.avatar_url"
                :alt="props.wikis.editor_user.username"
                size="sm"
                circle
              />
              <div class="text">
                <nuxt-link :to="'/user/' + props.wikis.editor_user.username" class="name">
                  <p>{{ props.wikis.editor_user.username }}</p>
                </nuxt-link>
                <p>您提交的页面正在审核中，请耐心等待</p>
              </div>
            </div>

            <div class="side-buttons">
              <button-styled color="red">
                <button @click="$refs.modal_confirm_given_up.show()">
                  <XIcon aria-hidden="true" />
                  放弃编辑
                </button>
              </button-styled>

              <button-styled color="green">
                <button style="margin-left: 5px" @click="$refs.modal_confirm_again.show()">
                  <CogIcon aria-hidden="true" />
                  重新编辑
                </button>
              </button-styled>
            </div>
          </div>
        </div>
      </div>
    </div>
    <!--    审核 -->
    <div
      v-if="
        props.wikis.is_editor &&
        !props.wikis.is_visitors &&
        !props.wikis.is_editor_user &&
        props.wikis.cache &&
        props.wikis.cache.status === 'review'
      "
    >
      <div class="markdown-body card">
        <h2>审核公开编辑百科</h2>
        <br />

        <div class="member">
          <div class="member-header">
            <div class="info">
              <Avatar
                :src="props.wikis.editor_user.avatar_url"
                :alt="props.wikis.editor_user.username"
                size="sm"
                circle
              />
              <div class="text">
                <nuxt-link :to="'/user/' + props.wikis.editor_user.username" class="name">
                  <p>{{ props.wikis.editor_user.username }}</p>
                </nuxt-link>
                <p>提交了百科页面</p>
              </div>
            </div>

            <div class="side-buttons">
              <button-styled color="green">
                <button @click="preReview">
                  <CogIcon aria-hidden="true" />
                  开始审核
                </button>
              </button-styled>
            </div>
          </div>
        </div>
        <div v-if="props.wikis.cache.message.length > 0" class="thread-summary">
          <div class="thread-title-row">
            <span class="thread-title">会话</span>
          </div>
          <div v-for="msg in props.wikis.cache.message" :key="msg.time">
            <div class="message has-body">
              <ConditionalNuxtLink
                class="message__icon"
                :is-link="!noLinks"
                :to="`/user/${msg.username}`"
                tabindex="-1"
                aria-hidden="true"
              >
                <Avatar class="message__icon" :src="msg.avatar_url" circle />
              </ConditionalNuxtLink>

              <span class="message__author">
                <ConditionalNuxtLink :is-link="true" :to="`/user/${msg.username}`">{{
                  msg.username
                }}</ConditionalNuxtLink>
              </span>
              <div
                class="message__body markdown-body"
                v-html="msg.message.replace('\n', '</br>')"
              />
              <span
                v-tooltip="$dayjs(msg.time).format('YYYY-MM-DD hh:mm:ss')"
                class="date"
                style="font-size: 13px"
              >
                {{ fromNow(msg.time) }}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div
      v-if="
        props.wikis.is_editor &&
        !props.wikis.is_visitors &&
        !props.wikis.is_editor_user &&
        !props.wikis.cache
      "
    >
      <div class="markdown-body card">
        <h2>公开编辑百科</h2>
        <br />

        <div class="member">
          <div class="member-header">
            <div class="info">
              <Avatar
                :src="props.wikis.editor_user.avatar_url"
                :alt="props.wikis.editor_user.username"
                size="sm"
                circle
              />
              <div class="text">
                <nuxt-link :to="'/user/' + props.wikis.editor_user.username" class="name">
                  <p>{{ props.wikis.editor_user.username }}</p>
                </nuxt-link>
                <p>正在编辑中</p>
              </div>
            </div>

            <div class="side-buttons">
              <button-styled color="green">
                <button disabled @click="preReview">
                  <CogIcon aria-hidden="true" />
                  用户提交后可审核
                </button>
              </button-styled>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div
      v-if="
        props.wikis.is_editor &&
        !props.wikis.is_visitors &&
        props.wikis.is_editor_user &&
        props.wikis.cache.status === 'draft'
      "
    >
      <div class="markdown-body card">
        <h2>编辑百科</h2>
        <br />

        <div class="member">
          <div class="member-header">
            <div class="info">
              <Avatar
                :src="props.wikis.editor_user.avatar_url"
                :alt="props.wikis.editor_user.username"
                size="sm"
                circle
              />
              <div class="text">
                <nuxt-link :to="'/user/' + props.wikis.editor_user.username" class="name">
                  <p>{{ props.wikis.editor_user.username }}</p>
                </nuxt-link>
                <p>正在编辑中</p>
              </div>
            </div>

            <div class="side-buttons">
              剩余编辑时间:
              {{ $dayjs(props.wikis.cache.again_time).add(5, "hour").diff($dayjs(), "minute") }}
              分钟
            </div>
          </div>
        </div>
        <div v-if="props.wikis.cache.message.length > 0" class="thread-summary">
          <div class="thread-title-row">
            <span class="thread-title">会话</span>
          </div>
          <div v-for="msg in props.wikis.cache.message" :key="msg.time">
            <div class="message has-body">
              <ConditionalNuxtLink
                class="message__icon"
                :is-link="!noLinks"
                :to="`/user/${msg.username}`"
                tabindex="-1"
                aria-hidden="true"
              >
                <Avatar class="message__icon" :src="msg.avatar_url" circle />
              </ConditionalNuxtLink>

              <span class="message__author">
                <ConditionalNuxtLink :is-link="true" :to="`/user/${msg.username}`">{{
                  msg.username
                }}</ConditionalNuxtLink>
              </span>
              <div
                class="message__body markdown-body"
                v-html="msg.message.replace('\n', '</br>')"
              />
              <span
                v-tooltip="$dayjs(msg.time).format('YYYY-MM-DD HH:mm:ss')"
                class="date"
                style="font-size: 13px"
              >
                {{ fromNow(msg.time) }}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div v-if="props.wikis.is_editor && props.wikis.is_visitors">
      <div class="markdown-body card">
        <h2>公开编辑百科</h2>
        <br />

        <div class="member">
          <div class="member-header">
            <div class="info">
              <Avatar
                :src="props.wikis.editor_user.avatar_url"
                :alt="props.wikis.editor_user.username"
                size="sm"
                circle
              />
              <div class="text">
                <nuxt-link :to="'/user/' + props.wikis.editor_user.username" class="name">
                  <p>{{ props.wikis.editor_user.username }}</p>
                </nuxt-link>
                <p>正在编辑中</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div
      class="markdown-body card"
      v-html="renderHighlightedString(wiki ? wiki.body : '未设置任何页面未主页')"
    />

    <div
      v-if="
        props.wikis.is_editor &&
        props.wikis.is_editor_user &&
        props.wikis.cache &&
        props.wikis.cache.status === 'draft' &&
        !wiki
      "
      class="markdown-body card"
    >
      <h2>百科未设置任何页面为主页</h2>
      <br />
      <span class="label__description">
        请在左边的目录中创建一个页面，并且将其设置为主页后，该页面将显示为你设置的页面的内容
      </span>
    </div>

    <!--    其他用户，百科正在编辑中，告知谁正在编辑   -->
    <div
      v-if="props.wikis.is_visitors && props.wikis.is_editor && project.wiki_open"
      class="markdown-body card"
    >
      <h2>公开编辑百科</h2>
      <span class="label__description">
        该资源的作者开启了公开编辑百科的功能，您可以在此处创建/修改百科页面
      </span>
      <br />
      <br />
      <span class="label__description">
        用户
        <router-link :to="'/user/' + props.wikis.editor_user.username" style="font-weight: bold">{{
          props.wikis.editor_user.username
        }}</router-link>
        正在编辑该资源的百科页面，暂时无法申请编辑
      </span>
      <br />
      <br />
      <button-styled color="green">
        <button disabled @click="$refs.modalConfirmCreate.show()">
          <PlusIcon aria-hidden="true" />
          申请编辑权限
        </button>
      </button-styled>
    </div>
    <div v-if="!props.wikis.is_editor" class="markdown-body card">
      <h2 v-if="currentMember">编辑百科</h2>
      <h2 v-else-if="project.wiki_open">公开编辑百科</h2>
      <h2 v-else>百科</h2>
      <br />

      <div v-if="currentMember">
        <span class="label__description"> 您有权限编辑该资源的百科，您可以在此处创建百科页面 </span>
        <br />
        <br />
        <button-styled color="green">
          <button @click="$refs.modalConfirmCreate.show()">
            <PlusIcon aria-hidden="true" />
            开始编辑
          </button>
        </button-styled>
      </div>
      <div v-else-if="project.wiki_open">
        <span class="label__description">
          该资源的作者开启了公开编辑百科的功能，您可以在此处创建/修改百科页面
        </span>
        <br />
        <br />
        <button-styled color="green">
          <button @click="$refs.modalConfirmCreate.show()">
            <PlusIcon aria-hidden="true" />
            申请编辑权限
          </button>
        </button-styled>
      </div>
      <div v-else>该资源没有开启公开编辑百科的功能，您可以向作者申请编辑权限</div>
    </div>
  </section>
</template>

<script setup>
import {
  Avatar,
  ButtonStyled,
  ConditionalNuxtLink,
  ConfirmModal,
  NewModal,
  ScrollablePanel,
} from "@modrinth/ui";
import { PlusIcon, CogIcon, XIcon, CheckIcon } from "@modrinth/assets";
import ConfirmModal2 from "@modrinth/ui/src/components/modal/ConfirmModal2.vue";
import { renderHighlightedString } from "~/helpers/highlight.js";
const auth = await useAuth();

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
  wikis: {
    type: Object,
    default() {
      return {};
    },
  },
});
const title = `${props.project.title} - WIKI`;
const description = `浏览 ${props.project.title} 个图片的WIKI页面`;
let wiki = ref(null);
const router = useNativeRouter();
const route = useNativeRoute();

const data = useNuxtApp();

useSeoMeta({
  title,
  description,
  ogTitle: title,
  ogDescription: description,
});

if (
  !props.wikis.is_visitors &&
  props.wikis.is_editor &&
  props.wikis.cache &&
  props.wikis.cache.status === "draft"
) {
  props.wikis.cache.cache.forEach((wiki_) => {
    if (wiki_.featured) {
      wiki = wiki_;
    }
    if (wiki_.child && wiki_.child.length > 0) {
      wiki_.child.forEach((wiki__) => {
        if (wiki__.featured) {
          wiki = wiki__;
        }
      });
    }
  });
} else {
  props.wikis.wikis.forEach((wiki_) => {
    if (wiki_.featured) {
      wiki = wiki_;
    }
    if (wiki_.child && wiki_.child.length > 0) {
      wiki_.child.forEach((wiki__) => {
        if (wiki__.featured) {
          wiki = wiki__;
        }
      });
    }
  });
}

async function submitCreateWiki() {
  if (auth.value.user) {
    try {
      await useBaseFetch(`project/${route.params.id}/wiki_edit_start`, {
        apiVersion: 3,
        method: "POST",
      });
      data.$notify({
        group: "main",
        title: "成功",
        text: "</br>您已成功申请创建百科页面,请在五小时内提交审核",
        type: "success",
      });
      router.push(`/project/${route.params.id}/wikis`);
      modalConfirmCreate.value.hide();
    } catch (err) {
      console.log(err);
      data.$notify({
        group: "main",
        title: "发生错误",
        text: err.data.description,
        type: "error",
      });
    }
  } else {
    // auth.login()
    data.$notify({
      group: "main",
      title: "未登录",
      text: "</br>请先登录或创建账号",
      type: "error",
    });
    router.push(`/auth/sign-in`);
  }
  // console.log('submitCreateWiki')
}

const preSortReview = ref([]);
const preBodyReview = ref([]);
const preADDReview = ref([]);
const preREMOVEReview = ref([]);
const preIndexSetReview = ref([]);
const preReviewWiki = ref();
const viewBodyWikiView = ref();
const viewBodyWiki = ref();
const viewBodyWikiIsChange = ref(false);
const rejectWikiCache = ref();
const modalConfirmCreate = ref();

const rejectWikiCacheMsg = ref("");

function viewBody(body) {
  viewBodyWikiIsChange.value = false;
  viewBodyWiki.value = body;
  preReviewWiki.value.hide();
  viewBodyWikiView.value.show();
}

function viewBodyChange(wiki) {
  viewBodyWikiIsChange.value = true;
  viewBodyWiki.value = wiki;
  preReviewWiki.value.hide();
  viewBodyWikiView.value.show();
}
function preReview() {
  // 第一步，获取到所有的新增的和被移除的WIKI
  const wikiNew = props.wikis.cache.cache;
  const wikiOld = props.wikis.wikis;

  wikiNew.forEach((wiki) => {
    if (!wiki.child) {
      wiki.child = [];
    }
  });
  wikiOld.forEach((wiki) => {
    if (!wiki.child) {
      wiki.child = [];
    }
  });

  const wikiNews = wikiNew.flatMap((x) => [x, ...x.child.map((child) => child)]);
  const wikiOlds = wikiOld.flatMap((x) => [x, ...x.child.map((child) => child)]);
  const wikiNewId = wikiNew.flatMap((x) => [x.id, ...x.child.map((child) => child.id)]);
  const wikiOldId = wikiOld.flatMap((x) => [x.id, ...x.child.map((child) => child.id)]);

  const commonIds = wikiNewId.filter((id) => wikiOldId.includes(id)); // 交集
  const addWiki = [...wikiNews].filter((wiki) => !wikiOldId.includes(wiki.id));
  const removedWiki = [...wikiOlds].filter((wiki) => !wikiNewId.includes(wiki.id));

  const commonObjects = new Map(
    wikiNews.map((wiki) => [wiki.id, [wikiOlds.find((oldWiki) => oldWiki.id === wiki.id), wiki]]),
  );

  preSortReview.value = [];
  preBodyReview.value = [];
  preADDReview.value = [];
  preREMOVEReview.value = [];
  preIndexSetReview.value = [];
  let featured = null;
  wikiOlds.forEach((wiki) => {
    if (wiki.featured) {
      featured = wiki;
    } else if (wiki.child && wiki.child.length > 0) {
      wiki.child.forEach((wiki__) => {
        if (wiki__.featured) {
          featured = wiki__;
        }
      });
    }
  });
  wikiNews.forEach((wiki) => {
    if (wiki.featured) {
      if (!featured) {
        preIndexSetReview.value.push({
          wiki_id: wiki.id,
          title: wiki.title,
          body: wiki.body,
        });
      } else if (featured && featured.id !== wiki.id) {
        preIndexSetReview.value.push({
          wiki_id: wiki.id,
          title: wiki.title,
          body: wiki.body,
        });
      }
    } else if (wiki.child && wiki.child.length > 0) {
      wiki.child.forEach((wiki__) => {
        if (wiki__.featured) {
          if (!featured) {
            preIndexSetReview.value.push({
              wiki_id: wiki__.id,
              title: wiki__.title,
              body: wiki__.body,
            });
          } else if (featured && featured.id !== wiki__.id) {
            preIndexSetReview.value.push({
              wiki_id: wiki__.id,
              title: wiki__.title,
              body: wiki__.body,
            });
          }
        }
      });
    }
  });

  commonObjects.forEach(([oldWiki, newWiki]) => {
    if (oldWiki && commonIds.includes(oldWiki.id)) {
      if (oldWiki.sort_order !== newWiki.sort_order) {
        preSortReview.value.push({
          wiki_id: oldWiki.id,
          title: newWiki.title,
          old_sort_order: oldWiki.sort_order,
          new_sort_order: newWiki.sort_order,
        });
      }
      if (oldWiki.body !== newWiki.body) {
        preBodyReview.value.push({
          wiki_id: oldWiki.id,
          title: newWiki.title,
          old_body: oldWiki.body,
          new_body: newWiki.body,
        });
      }
    }
  });
  addWiki.forEach((wiki) => {
    preADDReview.value.push({
      wiki_id: wiki.id,
      title: wiki.title,
      sort_order: wiki.sort_order,
      body: wiki.body,
    });
  });

  removedWiki.forEach((wiki) => {
    preREMOVEReview.value.push({
      wiki_id: wiki.id,
      title: wiki.title,
      sort_order: wiki.sort_order,
      body: wiki.body,
    });
  });

  preReviewWiki.value.show();
}

function preRejectWikiCache() {
  preReviewWiki.value.hide();
  rejectWikiCacheMsg.value = "";
  rejectWikiCache.value.show();
}

async function again() {
  try {
    await useBaseFetch(`project/${route.params.id}/wiki_submit_again/${props.wikis.cache.id}`, {
      apiVersion: 3,
      method: "POST",
    });
    data.$notify({
      group: "main",
      title: "成功",
      text: "重新编辑百科",
      type: "success",
    });
    // await read();
    router.push(`/project/${route.params.id}/wikis`);
  } catch (err) {
    data.$notify({
      group: "main",
      title: "发生错误",
      text: err.data.description,
      type: "error",
    });
  }
}

async function accept() {
  try {
    await useBaseFetch(`project/${route.params.id}/wiki_accept`, { apiVersion: 3, method: "POST" });
    data.$notify({
      group: "main",
      title: "成功",
      text: "已审核通过",
      type: "success",
    });
    // await read();
    router.push(`/project/${route.params.id}/wikis`);
  } catch (err) {
    data.$notify({
      group: "main",
      title: "发生错误",
      text: err.data.description,
      type: "error",
    });
  }
}
async function givenUp() {
  try {
    await useBaseFetch(`project/${route.params.id}/wiki_given_up/${props.wikis.cache.id}`, {
      apiVersion: 3,
      method: "POST",
    });
    data.$notify({
      group: "main",
      title: "成功",
      text: "已放弃本次提交",
      type: "success",
    });
    router.push(`/project/${route.params.id}/wikis`);
  } catch (err) {
    data.$notify({
      group: "main",
      title: "发生错误",
      text: err.data.description,
      type: "error",
    });
  }
}

async function confirmRejectWiki() {
  if (rejectWikiCacheMsg.value === "") {
    data.$notify({
      group: "main",
      title: "错误",
      text: "请填写拒绝理由",
      type: "error",
    });
    return;
  }

  rejectWikiCache.value.hide();
  try {
    await useBaseFetch(`project/${route.params.id}/wiki_reject`, {
      apiVersion: 3,
      method: "POST",
      body: { msg: rejectWikiCacheMsg.value },
    });
    data.$notify({
      group: "main",
      title: "成功",
      text: "您已成功拒绝该提交",
      type: "success",
    });
    router.push(`/project/${route.params.id}/wikis`);
  } catch (err) {
    console.log(err);
    data.$notify({
      group: "main",
      title: "发生错误",
      text: err.data.description,
      type: "error",
    });
  }
}
</script>

<style lang="scss" scoped>
.member {
  .member-header {
    display: flex;
    justify-content: space-between;
    .info {
      display: flex;
      .text {
        margin: auto 0 auto 0.5rem;
        font-size: var(--font-size-sm);

        .name {
          font-weight: bold;

          display: flex;
          align-items: center;
          gap: 0.25rem;

          svg {
            color: var(--color-orange);
          }
        }

        p {
          margin: 0.2rem 0;
        }
      }
    }
    .side-buttons {
      display: flex;
      align-items: center;
      .dropdown-icon {
        margin-left: 1rem;
        svg {
          transition: 150ms ease transform;
        }
      }
    }
  }
  .content {
    display: none;
    flex-direction: column;
    padding-top: var(--gap-md);
    .main-info {
      margin-bottom: var(--gap-lg);
    }
    .permissions {
      margin-bottom: var(--gap-md);
      max-width: 45rem;
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(10rem, 1fr));
      grid-gap: 0.5rem;
    }
  }
  &.open {
    .member-header {
      .dropdown-icon svg {
        transform: rotate(180deg);
      }
    }
    .content {
      display: flex;
    }
  }
}
:deep(.checkbox-outer) {
  button.checkbox {
    border: none;
  }
}

.thread-summary {
  display: flex;
  flex-direction: column;
  background-color: var(--color-bg);
  padding: var(--spacing-card-bg);
  border-radius: var(--size-rounded-card);
  border: 1px solid var(--color-divider-dark);
  gap: var(--spacing-card-sm);

  .thread-title-row {
    display: flex;
    flex-direction: row;
    align-items: center;

    .thread-title {
      font-weight: bold;
      color: var(--color-heading);
    }

    .thread-messages {
      margin-left: auto;
      color: var(--color-link);

      svg {
        vertical-align: top;
      }
    }
  }

  .thread-message {
    .user {
      font-weight: bold;
    }

    .date {
      color: var(--color-text-secondary);
      font-size: var(--font-size-sm);
    }
  }

  .thread-message,
  .thread-message > span {
    display: flex;
    flex-direction: row;
    gap: var(--spacing-card-xs);
    align-items: center;
  }

  &.raised {
    background-color: var(--color-raised-bg);
  }

  &:hover .thread-title-row,
  &:focus-visible .thread-title-row {
    text-decoration: underline;
    filter: var(--hover-filter);
  }

  &:active .thread-title-row {
    filter: var(--active-filter);
  }
}
.message {
  --gap-size: var(--spacing-card-xs);
  display: flex;
  flex-direction: row;
  gap: var(--gap-size);
  flex-wrap: wrap;
  align-items: center;
  border-radius: var(--size-rounded-card);
  padding: var(--spacing-card-md);
  word-break: break-word;

  .avatar,
  .backed-svg {
    --size: 1.5rem;
  }

  &.has-body {
    --gap-size: var(--spacing-card-sm);
    display: grid;
    grid-template:
      "icon author actions"
      "icon body actions"
      "date date date";
    grid-template-columns: min-content auto 1fr;
    column-gap: var(--gap-size);
    row-gap: var(--spacing-card-xs);

    .message__icon {
      margin-bottom: auto;
    }

    .avatar,
    .backed-svg {
      --size: 3rem;
    }
  }

  &:not(.no-actions):hover,
  &:not(.no-actions):focus-within {
    background-color: var(--color-table-alternate-row);

    .message__actions {
      opacity: 1;
    }
  }

  &.no-actions {
    padding: 0;

    .message__actions {
      display: none;
    }
  }
}

@media screen and (min-width: 600px) {
  .message {
    //grid-template:
    //  'icon author body'
    //  'date date date';
    //grid-template-columns: min-content auto 1fr;

    &.has-body {
      grid-template:
        "icon author actions"
        "icon body actions"
        "date date date";
      grid-template-columns: min-content auto 1fr;
    }
  }
}

@media screen and (min-width: 1024px) {
  .message {
    //grid-template: 'icon author body date';
    //grid-template-columns: min-content auto 1fr auto;

    &.has-body {
      grid-template:
        "icon author date actions"
        "icon body body actions";
      grid-template-columns: min-content auto 1fr;
      grid-template-rows: min-content 1fr auto;
    }
  }
}
</style>
