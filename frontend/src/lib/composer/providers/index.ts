import type { Component } from "svelte";
import DefaultEditor from "./DefaultEditor.svelte";
import XEditor from "./XEditor.svelte";
import LinkedInEditor from "./LinkedInEditor.svelte";
import FacebookEditor from "./FacebookEditor.svelte";
import RedditEditor from "./RedditEditor.svelte";
import InstagramEditor from "./InstagramEditor.svelte";
import ThreadsEditor from "./ThreadsEditor.svelte";
import BlueskyEditor from "./BlueskyEditor.svelte";
import MastodonEditor from "./MastodonEditor.svelte";
import TikTokEditor from "./TikTokEditor.svelte";

export interface ProviderEditorProps {
  content: string;
  onContentChange: (html: string) => void;
  integrationId: string;
}

export const providerEditors: Record<string, Component<ProviderEditorProps>> = {
  x: XEditor,
  linkedin: LinkedInEditor,
  "linkedin-page": LinkedInEditor,
  facebook: FacebookEditor,
  reddit: RedditEditor,
  instagram: InstagramEditor,
  "instagram-standalone": InstagramEditor,
  threads: ThreadsEditor,
  bluesky: BlueskyEditor,
  mastodon: MastodonEditor,
  tiktok: TikTokEditor,
};

export function getEditor(providerIdentifier: string): Component<ProviderEditorProps> {
  return providerEditors[providerIdentifier] || DefaultEditor;
}
