import { Component } from "react";
import type { ErrorInfo, ReactNode } from "react";

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("[ErrorBoundary] 컴포넌트 오류:", error, info.componentStack);
  }

  handleReset = () => {
    this.setState({ hasError: false, error: null });
  };

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }
      return (
        <div
          role="alert"
          style={{
            padding: "24px",
            textAlign: "center",
            color: "#c62828",
          }}
        >
          <h2>예기치 않은 오류가 발생했습니다</h2>
          <p style={{ fontSize: "0.875rem", color: "#555", marginBottom: "16px" }}>
            {this.state.error?.message}
          </p>
          <button onClick={this.handleReset}>다시 시도</button>
        </div>
      );
    }

    return this.props.children;
  }
}
