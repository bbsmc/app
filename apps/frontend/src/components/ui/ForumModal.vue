<template>
  <div class="forum-container">
    <!-- 帖子列表 -->
    <div class="posts-wrapper">
      <div v-for="post in forum" :id="`post-${post.floor_number}`" :key="post.floor_number" ref="postRefs"
        :data-floor-number="post.floor_number" class="card markdown-body">
        <div class="post-header">
          <!-- 用户信息区 -->
          <div class="user-info">
            <a :href="`/user/${post.user_name}`" target="_blank" class="user-avatar-link">
              <img :src="post.user_avatar" :alt="post.user_name" class="avatar" />
            </a>
            <a :href="`/user/${post.user_name}`" target="_blank" class="username">
              {{ post.user_name }}
            </a>
            <span class="post-time">{{ formatRelativeTime(post.created_at) }}</span>
          </div>
          <div class="post-id" :title="'点击复制链接'" @click="copyPostUrl(post.floor_number)">
            #{{ post.floor_number }}
          </div>
        </div>

        <!-- 如果是回复帖子，显示回复引用 -->
        <div v-if="post.reply_content" class="reply-reference" @click="scrollToPost(post.replied_to)">
          <div class="reply-info">
            <img :src="post.reply_content.user_avatar" :alt="post.reply_content.user_name" class="reply-avatar" />
            <div class="reply-user-info">
              <span class="reply-username">{{ post.reply_content.user_name }}</span>
              <span class="reply-post-id">#{{ post.replied_to }}</span>
            </div>
          </div>
          <div class="reply-quote" v-html="renderHighlightedString(post.reply_content.content)"></div>
        </div>

        <!-- 帖子内容 -->
        <div class="markdown-body" v-html="renderHighlightedString(post.content)" />

        <!-- 添加底部操作区 -->
        <div class="post-footer">
          <button class="reply-button" @click="showReplyForm(post)">回复</button>
        </div>

        <!-- 如果有回复，显示回复链接 -->
        <div v-if="post.replies.length > 0" class="replies-info">
          <span v-for="reply in post.replies" :key="reply" class="reply-link" @click="scrollToPost(reply.floor_number)"
            @mouseenter="showReplyPreview(reply)" @mouseleave="hideReplyPreview">
            #{{ reply.floor_number }}
          </span>
        </div>
      </div>
    </div>

    <!-- 时间线 -->
    <div v-if="!isMobile" class="timeline-indicator">
      <button class="timeline-reply-button" @click="showNewReply">
        <span>发表回复</span>
      </button>
      <div class="timeline-header" @click="scrollToFirst">
        <span>{{ formatMonthCN(firstPostDate) }}</span>
      </div>
      <div class="timeline-content">
        <div class="timeline-line">
          <div class="timeline-sections">
            <div v-for="(section, index) in timelineSections" :key="index" class="timeline-section"
              :title="`跳转到第 ${section.start + 1} - ${section.end + 1} 条`" @click="jumpToSection(index)"></div>
          </div>
        </div>
        <div class="timeline-position" :style="{ top: timelinePosition + '%' }">
          <div class="position-info">
            <div class="post-count">{{ currentPostNumber }}</div>
            <div class="post-date">{{ formatMonthCN(currentPost?.created_at) }}</div>
          </div>
        </div>
      </div>
      <div class="timeline-footer" @click="scrollToLast">
        <span>{{ formatMonthCN(lastPostDate) }}</span>
      </div>
    </div>
    <div v-else>
      <!-- 手机上隐藏时间线部分 -->
    </div>

    <!-- 添加悬浮提示框 -->
    <div v-if="previewPost" class="reply-preview" :style="previewPosition">
      <div class="preview-header">
        <img :src="previewPost.user_avatar" :alt="previewPost.user_name" class="preview-avatar" />
        <div class="preview-user-info">
          <span class="preview-username">{{ previewPost.user_name }}</span>
          <!-- <span class="preview-time">{{ formatRelativeTime(previewPost.created_at) }}</span> -->
        </div>
      </div>
      <div class="preview-content" v-html="renderHighlightedString(previewPost.content)"></div>
    </div>

    <!-- 修改回复表单为固定定位的底部弹出框 -->
    <div v-if="replyingTo" class="reply-form-overlay" @click.self="cancelReply">
      <div class="reply-form-modal">
        <div class="reply-form-header">
          <span>{{
            replyingTo.id === "new" ? "发表新帖" : `回复 #${replyingTo.floor_number}`
          }}</span>
          <button class="close-button" @click="cancelReply">×</button>
        </div>
        <div class="reply-form-content">
          <MarkdownEditor v-model="replyContent" :on-image-upload="onUploadHandler" placeholder="输入回复内容..." />
        </div>
        <div class="reply-form-actions">
          <button class="submit-button" :disabled="!replyContent.trim()" @click="submitReply">
            发送回复
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { debounce } from "lodash-es";
import { onMounted, ref, onUnmounted, computed, nextTick } from "vue";
import { useRoute, useRouter } from "vue-router";
import dayjs from "dayjs";
import { MarkdownEditor } from "@modrinth/ui";
import { renderHighlightedString } from "~/helpers/highlight.js";

const data = useNuxtApp();
const route = useRoute();
const router = useRouter();
const postRefs = ref([]);
const currentPostId = ref(null);

// 添加一个标志来控制观者是否应该更新 URL
const shouldUpdateUrl = ref(true);

// 添加一个变量来控制高亮状态
const highlightedPostId = ref(null);

// 添加一个变量来存储始的高亮定时器
let originalHighlightTimer = null;

// 添加一个函数来移除高亮效果
const removeHighlight = (postId) => {
  const element = document.getElementById(`post-${postId}`);
  if (element) {
    element.classList.remove("post-highlight");
    element.classList.remove("post-highlight-border");
  }
  highlightedPostId.value = null;
  window.removeEventListener("scroll", handleScroll);

  // 清定时器
  if (originalHighlightTimer) {
    clearTimeout(originalHighlightTimer);
    originalHighlightTimer = null;
  }
};

// 添加 isMobile 变量
const isMobile = ref(false); // 添加 isMobile 属性

const checkIfMobile = () => {
  isMobile.value = window.innerWidth <= 768; // 根据屏幕宽度判断是否为手机
};

// 修改滚动事件监听器
const handleScroll = () => {
  // 不再在滚动时重新设置定时器
  // 让原始时器继续运行直到结束
};

// 修改 createUrlObserver 函数
const createUrlObserver = () => {
  const options = {
    root: null,
    rootMargin: "-20% 0px -20% 0px",
    threshold: [0.5],
  };

  const observer = new IntersectionObserver((entries) => {
    if (!shouldUpdateUrl.value) return;

    const visibleEntries = entries
      .filter((entry) => entry.isIntersecting)
      .sort((a, b) => {
        const rectA = a.boundingClientRect;
        const rectB = b.boundingClientRect;
        const centerA = Math.abs(rectA.top + rectA.height / 2 - window.innerHeight / 2);
        const centerB = Math.abs(rectB.top + rectB.height / 2 - window.innerHeight / 2);
        return centerA - centerB;
      });

    if (visibleEntries.length > 0) {
      const floorNumber = visibleEntries[0].target.getAttribute("data-floor-number");
      if (floorNumber && route.query.id !== floorNumber) {
        router
          .replace({
            query: { ...route.query, id: floorNumber },
          })
          .catch((err) => console.error("Failed to update URL:", err));

        // 只更新 currentPostId，不触发其他操作
        const post = displayedPosts.value.find(
          (post) => post.floor_number.toString() === floorNumber,
        );
        if (post) {
          currentPostId.value = post.floor_number;
        }
      }
    }
  }, options);

  return observer;
};

let urlObserver = null;

// 添加响应式状态
const isLoading = ref(false);
const lastScrollPosition = ref(0);
const displayedPosts = ref([]);
const postsPerPage = 20;

// 添加一个变量来跟踪是否已加载所有数据
const allLoaded = ref({
  up: false,
  down: false,
});

// 修改 fetchPosts 函数
const fetchPosts = async (page) => {
  try {
    const pageNumber = parseInt(page) || 1;
    const params = new URLSearchParams({
      page_size: postsPerPage.toString(),
      page: pageNumber.toString(),
      // sort: 'floor_number' // 移除排序参数
    });

    const data = await useBaseFetch(`forum/${props.discussionId}/posts?${params}`, { apiVersion: 3, method: "GET" });

    // 确保返回的帖子按楼层号排序
    if (data?.posts) {
      data.posts.sort((a, b) => a.floor_number - b.floor_number);
    }

    return data;
  } catch (error) {
    console.error("Failed to fetch posts:", error);
    return null;
  }
};

// 修改初始化显示的帖子函数
const initDisplayPosts = async (targetId = null) => {
  allLoaded.value = { up: false, down: false };

  if (targetId) {
    // 如果有目标楼层号，计算对应的页码
    const targetPage = Math.ceil(parseInt(targetId) / postsPerPage);
    const data = await fetchPosts(targetPage);
    if (data) {
      displayedPosts.value = data.posts;
      totalPosts.value = data.pagination.total;

      // 更新当前帖子ID
      currentPostId.value = parseInt(targetId);

      // 滚动到目标楼层
      nextTick(() => {
        const element = document.querySelector(`[data-floor-number="${targetId}"]`);
        if (element) {
          element.scrollIntoView({ behavior: "smooth", block: "center" });
        }
      });
    }
  } else {
    // 没有目标楼层，加载第一页
    const data = await fetchPosts(1);
    if (data) {
      displayedPosts.value = data.posts;
      totalPosts.value = data.pagination.total;
      // 设置第一个帖子为当前帖子
      if (data.posts.length > 0) {
        currentPostId.value = data.posts[0].floor_number;
      }
    }
  }
};

// 添加观察新加载的帖子的函数
const observeNewPosts = () => {
  if (urlObserver) {
    const posts = document.querySelectorAll("[data-floor-number]");
    posts.forEach((post) => {
      urlObserver.observe(post);
    });
  }
};

// 修改加载更多帖子的函数
const loadMorePosts = async (direction = "down") => {
  if (isLoading.value) return;
  isLoading.value = true;

  try {
    const currentFloorNumber =
      direction === "down"
        ? displayedPosts.value[displayedPosts.value.length - 1].floor_number
        : displayedPosts.value[0].floor_number;

    const targetPage =
      direction === "down"
        ? Math.ceil((currentFloorNumber + 1) / postsPerPage)
        : Math.max(1, Math.floor((currentFloorNumber - 1) / postsPerPage));

    if (targetPage < 1 || targetPage > Math.ceil(totalPosts.value / postsPerPage)) {
      isLoading.value = false;
      return;
    }

    const data = await fetchPosts(targetPage);
    if (data?.posts.length > 0) {
      const oldHeight = document.documentElement.scrollHeight;
      const oldScrollTop = window.scrollY;

      const allPosts = [...displayedPosts.value];
      const newPosts = data.posts.filter(
        (post) => !allPosts.some((p) => p.floor_number === post.floor_number),
      );

      allPosts.push(...newPosts);
      displayedPosts.value = allPosts.sort((a, b) => a.floor_number - b.floor_number);

      // 向上加载时保持滚动位置
      nextTick(() => {
        if (direction === "up") {
          const newHeight = document.documentElement.scrollHeight;
          const heightDiff = newHeight - oldHeight;
          window.scrollTo(0, oldScrollTop + heightDiff);
        }
        // 重新观察新加载的帖子
        observeNewPosts();
      });
    }
  } finally {
    setTimeout(() => {
      isLoading.value = false;
    }, 200);
  }
};

onMounted(() => {
  const postId = route.query.id;

  // 初始化显示的帖子
  initDisplayPosts(postId).then(() => {
    if (postId) {
      shouldUpdateUrl.value = false; // 暂时禁用 URL 更新
      highlightPost(postId);

      // 如果是回复，也高亮被回复的帖子
      const targetPost = displayedPosts.value.find((post) => post.floor_number === postId);
      if (targetPost?.replied_to) {
        const replyToElement = document.getElementById(`post-${targetPost.replied_to}`);
        if (replyToElement) {
          replyToElement.classList.add("post-highlight-reply");
          setTimeout(() => {
            replyToElement.classList.remove("post-highlight-reply");
          }, 5000);
        }
      }

      // 1秒后启用 URL 更新
      setTimeout(() => {
        shouldUpdateUrl.value = true;
      }, 1000);
    } else {
      shouldUpdateUrl.value = true; // 如果没有指定帖子 ID，直接启用 URL 更新
    }

    // 观察所有帖子
    nextTick(() => {
      observeNewPosts();
    });

    // 检查是否为手机
    checkIfMobile(); // 添加检查手机的函数
  });

  // 添加滚动监听
  window.addEventListener("scroll", debouncedScroll);

  // 初始化 URL 观察
  urlObserver = createUrlObserver();
});

onUnmounted(() => {
  // 移除滚动监听
  window.removeEventListener("scroll", debouncedScroll);
  window.removeEventListener("scroll", handleScroll);
  if (urlObserver) {
    urlObserver.disconnect();
  }
  if (originalHighlightTimer) {
    clearTimeout(originalHighlightTimer);
  }
});

// 修改模中的 forum 绑定
const forum = computed(() => displayedPosts.value);

const props = defineProps({
  discussionId: {
    type: String,
    default: () => (null),
  },
});

// 添加复制链接功能
const copyPostUrl = (postId) => {
  const currentUrl = new URL(window.location.href);
  currentUrl.searchParams.set("id", postId);
  const url = currentUrl.toString();

  navigator.clipboard
    .writeText(url)
    .then(() => {
      // 可以添加个提示，表示复制成功
      // alert('链接已复制到剪贴板')
      data.$notify({
        group: "main",
        title: "成功",
        text: "</br>链接已复制到剪贴板",
        type: "success",
      });
    })
    .catch((err) => {
      console.error("复制失败:", err);
    });
};

// 添加计算属性
const currentPost = computed(() => {
  if (!currentPostId.value) return null;
  return forum.value.find((post) => post.floor_number === currentPostId.value);
});

const currentPostNumber = computed(() => {
  const currentNumber = currentPostId.value;
  const currentPage = Math.ceil(currentNumber / postsPerPage);
  const totalPages = Math.ceil(totalPosts.value / postsPerPage);
  return `${currentPage} / ${totalPages}`;
});

const timelinePosition = computed(() => {
  const currentNumber = currentPostId.value;
  const currentPage = Math.ceil(currentNumber / postsPerPage);
  const totalPages = Math.ceil(totalPosts.value / postsPerPage);
  return ((currentPage - 1) / (totalPages - 1)) * 100;
});

// 格式化份
const formatMonthCN = (dateString) => {
  if (!dateString) return "";
  const date = new Date(dateString);
  return `${date.getFullYear()}年${date.getMonth() + 1}月`;
};

// 添加相对时间格式化函数
const formatRelativeTime = (dateString) => {
  if (!dateString) return "";
  return dayjs(dateString).fromNow();
};

// 添加计算属性获取第一个和最后一个帖子的日期
const firstPostDate = computed(() => displayedPosts.value[0]?.created_at);
const lastPostDate = computed(
  () => displayedPosts.value[displayedPosts.value.length - 1]?.created_at,
);

// 修改其他函数使用 loadPage
const scrollToFirst = async () => {
  const data = await fetchPosts(1);

  if (data?.posts.length > 0) {
    displayedPosts.value = data.posts;
    nextTick(() => {
      const firstPost = data.posts[0];
      // 更新当前帖子ID，这会触发时间线位置的更新
      currentPostId.value = firstPost.floor_number;

      const element = document.getElementById(`post-${firstPost.floor_number}`);
      if (element) {
        element.scrollIntoView({ behavior: "smooth", block: "center" });
        router.replace({
          query: { ...route.query, id: firstPost.floor_number.toString() },
        });
      }
    });
  }
};

const scrollToLast = async () => {
  const lastPage = Math.ceil(totalPosts.value / postsPerPage);
  const data = await fetchPosts(lastPage);

  if (data?.posts.length > 0) {
    displayedPosts.value = data.posts;
    nextTick(() => {
      const lastPost = data.posts[data.posts.length - 1];
      // 更新当前帖子ID，这会触发时间线位置的更新
      currentPostId.value = lastPost.floor_number;

      const element = document.getElementById(`post-${lastPost.floor_number}`);
      if (element) {
        element.scrollIntoView({ behavior: "smooth", block: "center" });
        router.replace({
          query: { ...route.query, id: lastPost.floor_number.toString() },
        });
      }
    });
  }
};

const jumpToSection = (sectionIndex) => {
  const section = timelineSections.value[sectionIndex];
  const targetFloorNumber = Math.floor((section.start + section.end) / 2);
  const targetPage = Math.ceil(targetFloorNumber / postsPerPage);

  // 暂时禁用 URL 更新
  shouldUpdateUrl.value = false;

  fetchPosts(targetPage).then((data) => {
    if (data?.posts.length > 0) {
      displayedPosts.value = data.posts;
      nextTick(() => {
        const targetPost = data.posts.reduce((prev, curr) => {
          return Math.abs(curr.floor_number - targetFloorNumber) <
            Math.abs(prev.floor_number - targetFloorNumber)
            ? curr
            : prev;
        });
        const element = document.getElementById(`post-${targetPost.floor_number}`);
        if (element) {
          // 更新当前帖子ID，这会触发时间线位置的更新
          currentPostId.value = targetPost.floor_number;

          element.scrollIntoView({ behavior: "smooth", block: "center" });
          router.replace({
            query: { ...route.query, id: targetPost.floor_number.toString() },
          });

          // 重新初始化观察器
          if (urlObserver) {
            urlObserver.disconnect();
          }
          urlObserver = createUrlObserver();
          observeNewPosts();

          // 延迟重新启用 URL 更新
          setTimeout(() => {
            shouldUpdateUrl.value = true;
          }, 1000);
        }
      });
    }
  });
};

const scrollToPost = async (postId) => {
  hideReplyPreview();
  const targetPost = displayedPosts.value.find((p) => p.id === postId);

  if (targetPost) {
    // 如果目标帖子在当前显示的帖子中，直接滚动
    const element = document.getElementById(`post-${postId}`);
    if (element) {
      element.scrollIntoView({ behavior: "smooth", block: "center" });
      router.replace({
        query: { ...route.query, id: targetPost.floor_number.toString() },
      });
      highlightPost(postId);
    }
  } else {
    // 如果不在当前显示的帖子中，计算页码并加载
    const targetPage = Math.ceil(parseInt(postId) / postsPerPage);
    const data = await fetchPosts(targetPage);
    if (data?.posts) {
      displayedPosts.value = data.posts;
      nextTick(() => {
        const element = document.getElementById(`post-${postId}`);
        if (element) {
          element.scrollIntoView({ behavior: "smooth", block: "center" });
          const post = data.posts.find((p) => p.id === postId);
          if (post) {
            router.replace({
              query: { ...route.query, id: post.floor_number.toString() },
            });
          }
          highlightPost(postId);
        }
      });
    }
  }
};

// 添加预览相关的状态
const previewPost = ref(null);
const previewPosition = ref({
  top: "0px",
  left: "0px",
});

// 显示回复预览
const showReplyPreview = (reply) => {
  previewPost.value = reply;
  // 在下一个事件循环中更新位置，确保 DOM 已更新
  nextTick(() => {
    const target = event.target;
    const rect = target.getBoundingClientRect();
    const preview = document.querySelector(".reply-preview");
    if (preview) {
      const previewRect = preview.getBoundingClientRect();
      // 计算位置，确保预览框不会超出视口
      const left = Math.min(rect.left, window.innerWidth - previewRect.width - 20);
      const top = rect.bottom + window.scrollY + 10;
      previewPosition.value = {
        top: `${top}px`,
        left: `${left}px`,
      };
    }
  });
};

// 隐藏回复预览
const hideReplyPreview = () => {
  previewPost.value = null;
};

// 添加回复相关的状态
const replyingTo = ref(null);
const replyContent = ref("");

// 显示回复表单
const showReplyForm = (post) => {
  replyingTo.value = post;
  replyContent.value = "";
};

// 取消回复
const cancelReply = () => {
  replyingTo.value = null;
  replyContent.value = "";
};

// 修改发表新回复的函数
const showNewReply = () => {
  // 直接显示回复表单，不设置 replyingTo
  replyingTo.value = { id: "new" }; // 设置一个特殊值表示新帖子
  replyContent.value = "";
};

// 修改提交回复函数，处理新帖子和回复两种情况
const submitReply = () => {
  if (!replyContent.value.trim()) return;

  // 创建新帖子
  const newPost = {
    id: (totalPosts.value + 1).toString(),
    content: replyContent.value,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    user: {
      id: "1",
      name: "ModMaster",
      avatar:
        "https://cdn.bbsmc.net/bbsmc/data/rKJacHzy/79f0bf4c2173b64ddfeb0c1ce77682ca64a14ac3_96.webp",
    },
    replies: [],
    replied_to: replyingTo.value.id === "new" ? null : replyingTo.value.id,
    reply_content:
      replyingTo.value.id === "new"
        ? null
        : {
          user: replyingTo.value.user,
          content: replyingTo.value.content,
        },
  };

  // 如果是回复帖子，更新被回复帖子的 replies 数组
  if (replyingTo.value.id !== "new") {
    const targetPost = displayedPosts.value.find(
      (post) => post.floor_number === replyingTo.value.id,
    );
    if (targetPost) {
      targetPost.replies.push(newPost.floor_number);
    }
  }

  // 添加新帖子到显示列表
  displayedPosts.value.push(newPost);
  totalPosts.value++;

  // 清除回复表单
  cancelReply();

  // 更新当前帖子ID和URL
  currentPostId.value = newPost.floor_number;
  router.replace({
    query: { id: newPost.floor_number },
  });

  // 滚动到新帖子
  nextTick(() => {
    const element = document.getElementById(`post-${newPost.floor_number}`);
    if (element) {
      element.scrollIntoView({ behavior: "smooth", block: "center" });
      highlightPost(newPost.floor_number);
    }
  });
};

// 添加图片上传处理函数
const onUploadHandler = async (file) => {
  const response = await useImageUpload(file, {
    context: "project",
    projectID: props.discussionId,
  });
  return response.url;
};

// 计算时间线区间
const timelineSections = computed(() => {
  const sectionCount = 10;
  const sectionSize = Math.ceil(totalPosts.value / sectionCount);

  return Array.from({ length: sectionCount }, (_, index) => ({
    start: index * sectionSize + 1,
    end: Math.min((index + 1) * sectionSize, totalPosts.value),
  }));
});

// 添加高亮函数
const highlightPost = (postId, duration = 5000) => {
  const element = document.getElementById(`post-${postId}`);
  if (element) {
    // 移除旧的高亮
    if (highlightedPostId.value) {
      removeHighlight(highlightedPostId.value);
    }

    // 添加新的高亮
    highlightedPostId.value = postId;
    element.classList.add("post-highlight");
    element.classList.add("post-highlight-border");

    // 滚动到元素
    element.scrollIntoView({ behavior: "smooth", block: "center" });

    // 设置高亮移除定时器
    if (originalHighlightTimer) {
      clearTimeout(originalHighlightTimer);
    }
    originalHighlightTimer = setTimeout(() => {
      removeHighlight(postId);
    }, duration);
  }
};

// 添加总帖子数和当前页码的响应式引用
const totalPosts = ref(0);
// 添加无限滚动处理函数
const handleInfiniteScroll = () => {
  if (isLoading.value) return;

  const scrollTop = window.scrollY;
  const windowHeight = window.innerHeight;
  const documentHeight = document.documentElement.scrollHeight;

  // 向下滚动检测 - 距离底部100px时加载
  if (documentHeight - (scrollTop + windowHeight) < 100 && !allLoaded.value.down) {
    loadMorePosts("down");
  }
  // 向上滚动检测 - 当滚动到顶部附近时加载
  else if (scrollTop < 800 && !allLoaded.value.up) {
    // 增加阈值到800px
    loadMorePosts("up");
  }

  lastScrollPosition.value = scrollTop;
};

// 使用防抖包装滚动处理函数
const debouncedScroll = debounce(handleInfiniteScroll, 50); // 从100ms改为50ms
</script>

<style scoped>
.forum-container {
  display: flex;
  gap: 20px;
}

.posts-wrapper {
  flex: 1;
}

.timeline-indicator {
  position: sticky;
  top: 20px;
  height: calc(100vh - 120px);
  width: 100px;
  display: flex;
  flex-direction: column;
  font-size: 0.8em;
  color: #8f9ba8;
  margin-right: 20px;
}

.timeline-reply-button {
  background: #2d3139;
  border: none;
  color: #edeff1;
  padding: 8px 12px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.9em;
  margin-bottom: 12px;
  transition: background-color 0.2s ease;
  width: 100%;
}

.timeline-reply-button:hover {
  background: #363b44;
}

.timeline-header,
.timeline-footer {
  padding: 10px 0;
  text-align: center;
  color: #8f9ba8;
  font-size: 0.9em;
  cursor: pointer;
  transition: color 0.2s ease;
}

.timeline-header:hover,
.timeline-footer:hover {
  color: #edeff1;
}

.timeline-content {
  flex: 1;
  position: relative;
  cursor: default;
}

.timeline-line {
  position: absolute;
  left: 50%;
  top: 0;
  bottom: 0;
  width: 2px;
  background-color: #2d3139;
  z-index: 0;
}

.timeline-sections {
  position: absolute;
  left: -10px;
  right: -10px;
  top: 0;
  bottom: 0;
  display: flex;
  flex-direction: column;
  z-index: 1;
}

.timeline-section {
  flex: 1;
  cursor: pointer;
  transition: background-color 0.2s ease;
  position: relative;
}

.timeline-section:hover::after {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  top: 0;
  bottom: 0;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 4px;
}

.timeline-section:active::after {
  background: rgba(255, 255, 255, 0.15);
}

.timeline-position {
  position: absolute;
  left: 0;
  width: 100%;
  transform: translateY(-50%);
  transition: none;
  cursor: move;
  z-index: 2;
}

.timeline-position.is-dragging {
  cursor: move;
}

.position-info {
  background: #2d3139;
  padding: 8px;
  border-radius: 6px;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
  text-align: center;
  border: 1px solid #363b44;
  user-select: none;
  /* 防止拖拽时选中文本 */
  pointer-events: none;
  /* 防止信息框影响拖动 */
}

.post-count {
  font-weight: 500;
  margin-bottom: 4px;
  color: #edeff1;
}

.post-date {
  color: #8f9ba8;
  font-size: 0.9em;
}

.user-info {
  display: flex;
  align-items: center;
  margin-bottom: 0;
}

.avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  margin-right: 10px;
}

.username {
  font-weight: 500;
  margin-right: 10px;
}

.post-time {
  color: #666;
  font-size: 0.9em;
}

.post-highlight {
  animation: popEffect 5s ease-out;
}

.post-highlight-border {
  border-left: 3px solid #007bff;
}

@keyframes popEffect {
  0% {
    transform: scale(1);
  }

  50% {
    transform: scale(1.02);
  }

  100% {
    transform: scale(1);
  }
}

.post-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
}

.post-id {
  color: #666;
  font-size: 0.9em;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
  transition: color 0.2s ease;
}

.post-id:hover {
  color: #edeff1;
}

.markdown-body {
  padding: 16px;
}

/* 添加拖拽时的视觉反馈 */
.timeline-content:hover .timeline-line::before,
.timeline-content:active .timeline-line::before {
  display: none;
}

.reply-reference {
  margin: 8px 16px;
  padding: 8px 12px;
  background: #2d3139;
  border-radius: 4px;
  cursor: pointer;
  transition: background-color 0.2s ease;
}

.reply-reference:hover {
  background: #363b44;
}

.reply-info {
  display: flex;
  align-items: center;
  margin-bottom: 8px;
}

.reply-avatar {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  margin-right: 8px;
}

.reply-user-info {
  display: flex;
  align-items: center;
  gap: 8px;
}

.reply-username {
  color: #edeff1;
  font-size: 0.9em;
  font-weight: 500;
}

.reply-post-id {
  color: #8f9ba8;
  font-size: 0.9em;
}

.reply-quote {
  color: #edeff1;
  font-size: 0.95em;
  opacity: 0.8;
  overflow: hidden;
  max-height: 100px;
  margin-left: 32px;
  /* 与头像对齐 */
}

.replies-info {
  margin: 8px 16px;
  padding-top: 8px;
  border-top: 1px solid #2d3139;
  color: #8f9ba8;
  font-size: 0.9em;
}

.replies-info span {
  cursor: pointer;
  margin-right: 8px;
  transition: color 0.2s ease;
}

.replies-info span:hover {
  color: #edeff1;
}

.reply-preview {
  position: absolute;
  z-index: 1000;
  background: #2d3139;
  border: 1px solid #363b44;
  border-radius: 6px;
  padding: 12px;
  min-width: 300px;
  max-width: 500px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
}

.preview-header {
  display: flex;
  align-items: center;
  margin-bottom: 8px;
}

.preview-avatar {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  margin-right: 8px;
}

.preview-user-info {
  display: flex;
  flex-direction: column;
}

.preview-username {
  color: #edeff1;
  font-size: 0.9em;
  font-weight: 500;
}

.preview-time {
  color: #8f9ba8;
  font-size: 0.8em;
  margin-top: 2px;
}

.preview-content {
  color: #edeff1;
  font-size: 0.95em;
  line-height: 1.5;
  max-height: 200px;
  overflow-y: auto;
}

.reply-link {
  position: relative;
}

/* 优化滚动条样式 */
.preview-content::-webkit-scrollbar {
  width: 4px;
}

.preview-content::-webkit-scrollbar-track {
  background: #363b44;
}

.preview-content::-webkit-scrollbar-thumb {
  background: #8f9ba8;
  border-radius: 2px;
}

.post-footer {
  padding: 4px 16px;
  display: flex;
  justify-content: flex-end;
  margin-top: -8px;
  opacity: 0;
  /* 默认隐藏 */
  transition: opacity 0.2s ease;
}

/* 当鼠标悬停在帖子卡片上时显示回复按钮 */
.card:hover .post-footer {
  opacity: 1;
}

.reply-button {
  background: transparent;
  border: none;
  color: #8f9ba8;
  padding: 2px 12px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.9em;
  transition: color 0.2s ease;
}

.reply-button:hover {
  color: #edeff1;
}

.replies-info {
  margin: 8px 16px;
  padding-top: 8px;
  border-top: 1px solid #2d3139;
  color: #8f9ba8;
  font-size: 0.9em;
}

.reply-form {
  margin: 8px 16px;
  padding: 12px;
  background: #2d3139;
  border-radius: 4px;
  border: 1px solid #363b44;
}

.reply-form-header {
  color: #8f9ba8;
  font-size: 0.9em;
  margin-bottom: 8px;
}

.reply-textarea {
  width: 100%;
  min-height: 100px;
  background: #363b44;
  border: none;
  border-radius: 4px;
  padding: 8px;
  color: #edeff1;
  font-size: 0.95em;
  resize: vertical;
  margin-bottom: 8px;
}

.reply-textarea:focus {
  outline: none;
  box-shadow: 0 0 0 2px rgba(0, 123, 255, 0.25);
}

.reply-form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.cancel-button,
.submit-button {
  padding: 6px 12px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.9em;
  transition: all 0.2s ease;
}

.cancel-button {
  background: transparent;
  border: 1px solid #363b44;
  color: #8f9ba8;
}

.cancel-button:hover {
  background: #363b44;
  color: #edeff1;
}

.submit-button {
  background: #007bff;
  border: none;
  color: white;
}

.submit-button:hover:not(:disabled) {
  background: #0056b3;
}

.submit-button:disabled {
  background: #363b44;
  cursor: not-allowed;
  opacity: 0.7;
}

.reply-form-overlay {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  top: 0;
  background: rgba(0, 0, 0, 0.5);
  z-index: 1000;
  display: flex;
  align-items: flex-end;
  justify-content: center;
  /* 水平居中 */
  animation: fadeIn 0.2s ease;
}

.reply-form-modal {
  background: #26292f;
  width: 800px;
  /* 设置固定宽度 */
  max-width: 90%;
  /* 在小屏幕上自适应 */
  margin: 0 auto;
  /* 水平居中 */
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  animation: slideUp 0.3s ease;
  border-top-left-radius: 8px;
  border-top-right-radius: 8px;
}

.reply-form-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px;
  border-bottom: 1px solid #2d3139;
  color: #edeff1;
  font-size: 1em;
}

.close-button {
  background: transparent;
  border: none;
  color: #8f9ba8;
  font-size: 1.5em;
  cursor: pointer;
  padding: 0 8px;
  transition: color 0.2s ease;
}

.close-button:hover {
  color: #edeff1;
}

.reply-form-content {
  flex: 1;
  min-height: 200px;
  max-height: calc(80vh - 120px);
  overflow-y: auto;
  padding: 16px;
}

.reply-form-actions {
  padding: 16px;
  display: flex;
  justify-content: flex-end;
  border-top: 1px solid #2d3139;
}

.submit-button {
  background: #007bff;
  border: none;
  color: white;
  padding: 8px 24px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.95em;
  transition: background-color 0.2s ease;
}

.submit-button:hover:not(:disabled) {
  background: #0056b3;
}

.submit-button:disabled {
  background: #363b44;
  cursor: not-allowed;
  opacity: 0.7;
}

@keyframes fadeIn {
  from {
    opacity: 0;
  }

  to {
    opacity: 1;
  }
}

@keyframes slideUp {
  from {
    transform: translateY(100%);
  }

  to {
    transform: translateY(0);
  }
}

/* 移除旧的回复表单样式 */
.reply-form {
  display: none;
}

.user-avatar-link {
  display: block;
  text-decoration: none;
  transition: opacity 0.2s ease;
}

.user-avatar-link:hover {
  opacity: 0.8;
}

.username {
  font-weight: 500;
  margin-right: 10px;
  color: #edeff1;
  text-decoration: none;
  transition: color 0.2s ease;
}

.username:hover {
  color: #edeff1;
}

.post-highlight-reply {
  border-left: 3px solid #ffd700;
  /* 使用金色来区分被回复的帖子 */
  animation: popEffect 5s ease-out;
}
</style>
