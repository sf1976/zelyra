export async function clickStudioMenuAction(page, menu, target) {
  await page.locator(`[data-studio-menu="${menu}"]`).click();
  await page.locator(`.studio-menu-popover:not([hidden]) [data-menu-target="${target}"]`).click();
}
