<template>
  <section class="normal-page__content">
    <div
      v-if="wiki.body"
      class="markdown-body card"
      v-html="renderHighlightedString(wiki.body || '')"
    />
  </section>
</template>


<script setup>

import { renderHighlightedString } from "~/helpers/highlight.js";

const props = defineProps({
  project: {
    type: Object,
    default() {
      return {};
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
const route = useNativeRoute();
useSeoMeta({
  title,
  description,
  ogTitle: title,
  ogDescription: description,
});

// console.log('wikis', props.wikis);
if (props.wikis.is_editor === true) {
  props.wikis.cache.cache.forEach((wiki_) => {
    if (wiki_.child){
      wiki_.child.forEach((wiki__) => {
        if (route.params.wiki === wiki__.slug){
          wiki = wiki__;
        }
      });
    }
    if (route.params.wiki === wiki_.slug){
      wiki = wiki_;
    }
  });
}else {
  props.wikis.wikis.forEach((wiki_) => {
    if (wiki_.child){
      wiki_.child.forEach((wiki__) => {
        if (route.params.wiki === wiki__.slug){
          wiki = wiki__;
        }
      });
    }
    if (route.params.wiki === wiki_.slug){
      wiki = wiki_;
    }
  });
}



</script>

<style scoped lang="scss">

</style>