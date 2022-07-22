///<reference path="server/index.d.ts"/>

import { expect, test } from "@playwright/test";

const url = "http://localhost:8085";

test.beforeEach(async ({ page }) => {
  await page.goto(url);
});

test.describe("PublicDirectory", () => {
  test("lookupNode can fetch file added to directory", async ({ page }) => {
    const result = await page.evaluate(async () => {
      const {
        wnfs: { PublicDirectory },
        mock: { MemoryBlockStore, sampleCID },
      } = await window.setup();

      const time = new Date();
      const store = new MemoryBlockStore();
      const root = new PublicDirectory(time);

      var { rootDir } = await root.write(["text.txt"], sampleCID, time, store);

      return await rootDir.lookupNode("text.txt", store);
    });

    expect(result).toBeDefined();
  });

  test("lookupNode cannot fetch file not added to directory", async ({
    page,
  }) => {
    const result = await page.evaluate(async () => {
      const {
        wnfs: { PublicDirectory },
        mock: { MemoryBlockStore },
      } = await window.setup();

      const time = new Date();
      const store = new MemoryBlockStore();
      const root = new PublicDirectory(time);

      return await root.lookupNode("Unknown", store);
    });

    expect(result).toBe(undefined);
  });

  test("mkdir can create new directory", async ({ page }) => {
    const result = await page.evaluate(async () => {
      const {
        wnfs: { PublicDirectory },
        mock: { MemoryBlockStore, sampleCID },
      } = await window.setup();

      const time = new Date();
      const store = new MemoryBlockStore();
      const root = new PublicDirectory(time);

      var { rootDir } = await root.mkdir(["pictures", "cats"], time, store);

      var { rootDir } = await rootDir.write(
        ["pictures", "cats", "tabby.png"],
        sampleCID,
        time,
        store
      );

      var { rootDir } = await rootDir.getNode(
        ["pictures", "cats", "tabby.png"],
        store
      );

      return rootDir;
    });

    expect(result).toBeDefined();
  });

  test("ls can list children under directory", async ({ page }) => {
    const result = await page.evaluate(async () => {
      const {
        wnfs: { PublicDirectory },
        mock: { MemoryBlockStore, sampleCID },
      } = await window.setup();

      const time = new Date();
      const store = new MemoryBlockStore();
      const root = new PublicDirectory(time);

      var { rootDir } = await root.mkdir(["pictures", "dogs"], time, store);

      var { rootDir } = await rootDir.write(
        ["pictures", "cats", "tabby.png"],
        sampleCID,
        time,
        store
      );

      var { result } = await rootDir.ls(["pictures"], store);

      return result;
    });

    expect(result.length).toBe(2);
    expect(result[0].name).toBe("cats");
    expect(result[1].name).toBe("dogs");
  });

  test("rm can remove children from directory", async ({ page }) => {
    const result = await page.evaluate(async () => {
      const {
        wnfs: { PublicDirectory },
        mock: { MemoryBlockStore, sampleCID },
      } = await window.setup();

      const time = new Date();
      const store = new MemoryBlockStore();
      const root = new PublicDirectory(time);

      var { rootDir } = await root.write(
        ["pictures", "dogs", "billie.jpeg"],
        sampleCID,
        time,
        store
      );
      var { rootDir } = await rootDir.write(
        ["pictures", "cats", "tabby.png"],
        sampleCID,
        time,
        store
      );
      var { rootDir } = await rootDir.rm(["pictures", "cats"], store);
      var { result } = await rootDir.ls(["pictures"], store);

      return result;
    });

    expect(result.length).toEqual(1)
    expect(result[0].name).toEqual("dogs");
  });
});
