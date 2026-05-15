import DefaultEditor from "./DefaultEditor.svelte";
import XEditor from "./XEditor.svelte";
import LinkedInEditor from "./LinkedInEditor.svelte";
import FacebookEditor from "./FacebookEditor.svelte";

export interface ProviderEditorProps {
  content: string;
  onContentChange: (html: string) => void;
  integrationId: string;
}

export const providerEditors: Record<string, any> = {
  x: XEditor,
  linkedin: LinkedInEditor,
  "linkedin-page": LinkedInEditor,
  facebook: FacebookEditor,
};

export function getEditor(providerIdentifier: string): any {
  return providerEditors[providerIdentifier] || DefaultEditor;
}
