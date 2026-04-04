// @vitest-environment jsdom
import React from "react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { ErrorBoundary } from "../../ErrorBoundary";

function BrokenChild(): React.ReactNode {
  throw new Error("테스트 에러");
}

function GoodChild() {
  return <div>정상 컴포넌트</div>;
}

describe("ErrorBoundary", () => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let consoleSpy: any;

  beforeEach(() => {
    // Suppress expected console.error output from React during error boundary tests
    consoleSpy = vi.spyOn(console, "error").mockImplementation(() => undefined);
  });

  afterEach(() => {
    consoleSpy.mockRestore();
  });

  it("자식 컴포넌트가 정상이면 그대로 렌더링된다", () => {
    render(
      <ErrorBoundary>
        <GoodChild />
      </ErrorBoundary>,
    );
    expect(screen.getByText("정상 컴포넌트")).toBeInTheDocument();
  });

  it("자식 컴포넌트 에러 시 fallback UI를 표시한다", () => {
    render(
      <ErrorBoundary>
        <BrokenChild />
      </ErrorBoundary>,
    );
    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByText("예기치 않은 오류가 발생했습니다")).toBeInTheDocument();
    expect(screen.getByText("테스트 에러")).toBeInTheDocument();
  });

  it("커스텀 fallback prop을 사용할 수 있다", () => {
    render(
      <ErrorBoundary fallback={<div>커스텀 에러 화면</div>}>
        <BrokenChild />
      </ErrorBoundary>,
    );
    expect(screen.getByText("커스텀 에러 화면")).toBeInTheDocument();
  });

  it("다시 시도 버튼 클릭 시 에러 상태가 초기화된다", () => {
    let shouldThrow = true;
    function MaybeThrow() {
      if (shouldThrow) throw new Error("에러 발생");
      return <div>복구됨</div>;
    }

    const { rerender } = render(
      <ErrorBoundary>
        <MaybeThrow />
      </ErrorBoundary>,
    );

    expect(screen.getByRole("alert")).toBeInTheDocument();

    shouldThrow = false;
    fireEvent.click(screen.getByText("다시 시도"));

    rerender(
      <ErrorBoundary>
        <MaybeThrow />
      </ErrorBoundary>,
    );

    expect(screen.getByText("복구됨")).toBeInTheDocument();
  });
});
