// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

#pragma once

#include "AcpConnection.g.h"

#include <mutex>
#include <condition_variable>
#include <future>

#include "BaseTerminalConnection.h"

namespace winrt::Microsoft::Terminal::TerminalConnection::implementation
{
    struct AcpConnection : AcpConnectionT<AcpConnection>, BaseTerminalConnection<AcpConnection>
    {
        AcpConnection() = default;
        void Initialize(const Windows::Foundation::Collections::ValueSet& settings);

        void Start();
        void WriteInput(const winrt::array_view<const char16_t> buffer);
        void Resize(uint32_t rows, uint32_t columns);
        void Close() noexcept;

        til::event<TerminalOutputHandler> TerminalOutput;

    private:
        til::CoordType _initialRows{};
        til::CoordType _initialCols{};

        // Agent subprocess (raw pipes, NOT ConPTY)
        wil::unique_process_information _agentProcess;
        wil::unique_hfile _pipeToAgent; // write end -> agent's stdin
        wil::unique_hfile _pipeFromAgent; // read end <- agent's stdout
        std::mutex _writeMutex; // protects writes to _pipeToAgent

        // Background threads
        wil::unique_handle _hOutputThread;

        // JSON-RPC state
        std::atomic<int64_t> _nextRequestId{ 0 };
        struct PendingRequest
        {
            std::promise<Windows::Data::Json::JsonObject> promise;
        };
        std::mutex _pendingMutex;
        std::unordered_map<int64_t, PendingRequest> _pendingRequests;

        // User input collection (follows AzureConnection pattern)
        enum class InputMode
        {
            None = 0,
            Line
        };
        InputMode _currentInputMode{ InputMode::None };
        std::wstring _userInput;
        std::condition_variable _inputEvent;
        std::mutex _inputMutex;

        // ACP session state
        winrt::hstring _agentCliPath;
        winrt::hstring _workingDirectory;
        winrt::hstring _acpSessionId;
        winrt::hstring _initialPrompt;
        std::atomic<bool> _agentStreaming{ false };

        // Managed terminals for terminal/create requests
        static void _closePseudoConsole(HPCON hPC) noexcept;
        struct AcpManagedTerminal
        {
            std::string id;
            wil::unique_process_information process;
            wil::unique_hfile pipeRead;
            wil::unique_any<HPCON, decltype(&_closePseudoConsole), _closePseudoConsole> hPC;
            std::string outputBuffer;
            std::mutex outputMutex;
            bool exited{ false };
            DWORD exitCode{ 0 };
            wil::unique_event exitEvent{ wil::EventOptions::ManualReset };
            wil::unique_handle outputThread;
        };
        std::mutex _terminalsMutex;
        std::unordered_map<std::string, std::unique_ptr<AcpManagedTerminal>> _managedTerminals;
        int _nextTerminalId{ 1 };

        // Core methods
        DWORD _OutputThread();
        void _LaunchAgentProcess();
        void _SendJsonRpc(const winrt::hstring& method,
                          const Windows::Data::Json::JsonObject& params,
                          std::optional<int64_t> id = std::nullopt);
        std::future<Windows::Data::Json::JsonObject> _SendRequest(const winrt::hstring& method,
                                                                   const Windows::Data::Json::JsonObject& params);
        void _SendResponse(int64_t id, const Windows::Data::Json::JsonObject& result);
        void _SendErrorResponse(int64_t id, int code, const winrt::hstring& message);

        // Protocol flow
        void _DoHandshake();
        void _PromptLoop();

        // Message routing (called from reader thread)
        void _RouteMessage(const Windows::Data::Json::JsonObject& msg);
        void _HandleNotification(const winrt::hstring& method, const Windows::Data::Json::JsonObject& params);
        void _HandleAgentRequest(const winrt::hstring& method, const Windows::Data::Json::JsonObject& params, int64_t id);

        // Session update handlers
        void _HandleAgentMessageChunk(const Windows::Data::Json::JsonObject& update);
        void _HandleToolCall(const Windows::Data::Json::JsonObject& update);
        void _HandleToolCallUpdate(const Windows::Data::Json::JsonObject& update);
        void _HandlePlan(const Windows::Data::Json::JsonObject& update);

        // Agent request handlers
        void _HandleTerminalCreate(const Windows::Data::Json::JsonObject& params, int64_t id);
        void _HandleTerminalOutput(const Windows::Data::Json::JsonObject& params, int64_t id);
        void _HandleTerminalWaitForExit(const Windows::Data::Json::JsonObject& params, int64_t id);
        void _HandleTerminalKill(const Windows::Data::Json::JsonObject& params, int64_t id);
        void _HandleTerminalRelease(const Windows::Data::Json::JsonObject& params, int64_t id);
        void _HandleRequestPermission(const Windows::Data::Json::JsonObject& params, int64_t id);

        // VT output helpers
        void _WriteStringWithNewline(const std::wstring_view str);
        void _WriteVt(const std::wstring_view str);
        void _WritePromptIndicator();

        // Input helpers
        void _writeInput(const std::wstring_view data);
        std::optional<std::wstring> _ReadUserInput(InputMode mode);

        // Managed terminal helpers
        static DWORD WINAPI _ManagedTerminalOutputThread(LPVOID lpParameter);

    };
}

namespace winrt::Microsoft::Terminal::TerminalConnection::factory_implementation
{
    BASIC_FACTORY(AcpConnection);
}
