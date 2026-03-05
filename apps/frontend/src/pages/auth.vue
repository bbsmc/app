<template>
  <NuxtPage class="auth-container universal-card" />
</template>

<style>
.auth-container {
  width: 26rem;
  max-width: calc(100% - 2rem);
  margin: 1rem auto;
  display: flex;
  flex-direction: column;
  gap: 2rem;
}

.auth-container h1 {
  font-size: var(--font-size-xl);
  margin: 0 0 -1rem 0;
  color: var(--color-contrast);
}

.auth-container p {
  margin: 0;
}

.auth-container .btn {
  font-weight: 700;
  min-height: 2.5rem;
  text-decoration: none;
}

.centered-btn {
  margin-inline: auto;
}

.auth-form .full-width-btn {
  width: 100%;
  display: flex;
  justify-content: center;
}

.btn.continue-btn svg {
  margin: 0 0 0 0.5rem;
}

.auth-divider {
  display: flex;
  align-items: center;
  gap: 1rem;
  color: var(--color-secondary);
  font-size: var(--font-size-sm);
  margin: -0.5rem 0;
}

.auth-divider::before,
.auth-divider::after {
  content: "";
  flex: 1;
  height: 1px;
  background: var(--color-button-bg);
}

.separator-dot {
  color: var(--color-secondary);
  user-select: none;
}

.third-party {
  display: flex;
  flex-direction: column;
  gap: var(--gap-sm);
  width: 100%;
}

.third-party .btn.sso-btn {
  width: 100%;
  display: flex;
  justify-content: center;
  border: 1px solid var(--color-button-bg);
  background: transparent;
  transition: background 0.15s ease;
}

.third-party .btn.sso-btn:hover {
  background: var(--color-button-bg);
}

.third-party .btn svg {
  margin-right: var(--gap-sm);
  width: 1.25rem;
  height: 1.25rem;
}

.auth-form {
  display: flex;
  flex-direction: column;
  gap: var(--gap-md);
}

.auth-form .auth-form__input {
  width: 100%;
  flex-basis: auto;
}

.auth-form__additional-options {
  align-items: center;
  display: flex;
  justify-content: center;
  gap: var(--gap-md);
  flex-wrap: wrap;
}

.legal-notice {
  text-align: center;
  font-size: var(--font-size-xs);
  color: var(--color-secondary);
  margin: 0;
}

.legal-notice a {
  color: var(--color-secondary);
  text-decoration: underline;
  text-underline-offset: 2px;
}

.legal-notice a:hover {
  color: var(--color-contrast);
}
</style>
