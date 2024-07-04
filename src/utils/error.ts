export type IpcErrorType =
  | 'CreateClient'
  | 'Connect'
  | 'Authorize'
  | 'Subscribe'
  | 'EventReceive'
  | 'EventSend'
  | 'EventEncode';

export type IpcError = {
  error_type: IpcErrorType;
  message: string;
  payload?: unknown;
};

export type AuthErrorType = 'TokenFetch' | 'RefreshToken' | 'ConfigRead' | 'ConfigSave' | 'Decode' | 'IpcSend';

export type AuthError = {
  error_type: AuthErrorType;
  message: string;
};

export type RustError = IpcError | AuthError;
