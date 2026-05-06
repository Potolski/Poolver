/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/poolver_yield_defi.json`.
 */
export type PoolverYieldDefi = {
  "address": "DAitPF7KHzRDVWcV4XM3J7dYGrKJkH332dQHPYUiP7UP",
  "metadata": {
    "name": "poolverYieldDefi",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "Poolver V1 Tier 1 yield adapter (Kamino mock — SPEC_QUESTION-19/20)"
  },
  "docs": [
    "Tier 1 yield adapter — the Kamino-mock adapter. Same",
    "instruction surface as `poolver-yield-vault` (`initialize_adapter`,",
    "`deposit`, `withdraw`, `harvest`, `emergency_unwind`) so",
    "`poolver-core` can dispatch on `pool.tier` against a single CPI",
    "shape (arch §13 common interface; INV-21).",
    "",
    "SPEC_QUESTION-19 / SPEC_QUESTION-20: this V1 build does NOT CPI",
    "into real Kamino. The deployed (75%) leg is simulated via an",
    "internal token transfer between two PDA-owned USDC token accounts;",
    "\"yield\" is injected directly via the dev-only `mock_inject_yield`",
    "helper (gated by the `mock-yield` Cargo feature). Every site that",
    "real Kamino would replace is annotated with",
    "`// SPEC_QUESTION-19:` so a future engineer can grep them in one",
    "pass when the integration lands.",
    "",
    "SPEC_QUESTION-23: the oracle-deviation breaker input is also",
    "mocked (`mock_set_oracle_deviation`); production reads from a Pyth",
    "USDC/USD price feed."
  ],
  "instructions": [
    {
      "name": "deposit",
      "discriminator": [
        242,
        35,
        198,
        137,
        82,
        225,
        242,
        182
      ],
      "accounts": [
        {
          "name": "coreInvoker",
          "signer": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  114,
                  101,
                  95,
                  105,
                  110,
                  118,
                  111,
                  107,
                  101,
                  114
                ]
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                21,
                124,
                210,
                15,
                87,
                165,
                71,
                210,
                184,
                103,
                161,
                100,
                247,
                42,
                86,
                99,
                193,
                2,
                253,
                253,
                33,
                8,
                67,
                5,
                191,
                239,
                18,
                109,
                97,
                46,
                112,
                139
              ]
            }
          }
        },
        {
          "name": "adapterState",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  102,
                  105,
                  95,
                  97,
                  100,
                  97,
                  112,
                  116,
                  101,
                  114
                ]
              },
              {
                "kind": "account",
                "path": "adapter_state.pool",
                "account": "defiAdapterState"
              }
            ]
          }
        },
        {
          "name": "adapterUsdcVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  102,
                  105,
                  95,
                  97,
                  100,
                  97,
                  112,
                  116,
                  101,
                  114,
                  95,
                  117,
                  115,
                  100,
                  99
                ]
              },
              {
                "kind": "account",
                "path": "adapter_state.pool",
                "account": "defiAdapterState"
              }
            ]
          }
        },
        {
          "name": "sourceUsdc",
          "docs": [
            "Source of funds. Core passes the pool's PoolUsdcVault here.",
            "Same trade as Tier 0: we don't constrain `source_usdc.owner`",
            "because core handles that on its side (arch §5.1)."
          ],
          "writable": true
        },
        {
          "name": "sourceAuthority",
          "docs": [
            "Authority over `source_usdc`. In production this is the pool",
            "USDC vault PDA owned by core; passed in raw because Anchor",
            "can't type a foreign-program PDA here.",
            "at CPI time; no further validation needed in this adapter."
          ],
          "signer": true
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "adapterKtokenVault",
          "docs": [
            "Tier-1-specific: the kToken vault (mocked as a USDC token",
            "account in V1 — SPEC_QUESTION-19). The 75% deployed leg lands",
            "here via an internal token transfer that simulates the Kamino",
            "supply CPI."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  102,
                  105,
                  95,
                  97,
                  100,
                  97,
                  112,
                  116,
                  101,
                  114,
                  95,
                  107,
                  116,
                  111,
                  107,
                  101,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "adapter_state.pool",
                "account": "defiAdapterState"
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "emergencyUnwind",
      "discriminator": [
        137,
        171,
        84,
        125,
        152,
        107,
        49,
        248
      ],
      "accounts": [
        {
          "name": "coreInvoker",
          "signer": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  114,
                  101,
                  95,
                  105,
                  110,
                  118,
                  111,
                  107,
                  101,
                  114
                ]
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                21,
                124,
                210,
                15,
                87,
                165,
                71,
                210,
                184,
                103,
                161,
                100,
                247,
                42,
                86,
                99,
                193,
                2,
                253,
                253,
                33,
                8,
                67,
                5,
                191,
                239,
                18,
                109,
                97,
                46,
                112,
                139
              ]
            }
          }
        },
        {
          "name": "adapterState",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  102,
                  105,
                  95,
                  97,
                  100,
                  97,
                  112,
                  116,
                  101,
                  114
                ]
              },
              {
                "kind": "account",
                "path": "adapter_state.pool",
                "account": "defiAdapterState"
              }
            ]
          }
        },
        {
          "name": "adapterUsdcVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  102,
                  105,
                  95,
                  97,
                  100,
                  97,
                  112,
                  116,
                  101,
                  114,
                  95,
                  117,
                  115,
                  100,
                  99
                ]
              },
              {
                "kind": "account",
                "path": "adapter_state.pool",
                "account": "defiAdapterState"
              }
            ]
          }
        },
        {
          "name": "destinationUsdc",
          "writable": true
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "adapterKtokenVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  102,
                  105,
                  95,
                  97,
                  100,
                  97,
                  112,
                  116,
                  101,
                  114,
                  95,
                  107,
                  116,
                  111,
                  107,
                  101,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "adapter_state.pool",
                "account": "defiAdapterState"
              }
            ]
          }
        }
      ],
      "args": []
    },
    {
      "name": "harvest",
      "discriminator": [
        228,
        241,
        31,
        182,
        53,
        169,
        59,
        199
      ],
      "accounts": [
        {
          "name": "coreInvoker",
          "signer": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  114,
                  101,
                  95,
                  105,
                  110,
                  118,
                  111,
                  107,
                  101,
                  114
                ]
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                21,
                124,
                210,
                15,
                87,
                165,
                71,
                210,
                184,
                103,
                161,
                100,
                247,
                42,
                86,
                99,
                193,
                2,
                253,
                253,
                33,
                8,
                67,
                5,
                191,
                239,
                18,
                109,
                97,
                46,
                112,
                139
              ]
            }
          }
        },
        {
          "name": "adapterState",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  102,
                  105,
                  95,
                  97,
                  100,
                  97,
                  112,
                  116,
                  101,
                  114
                ]
              },
              {
                "kind": "account",
                "path": "adapter_state.pool",
                "account": "defiAdapterState"
              }
            ]
          }
        },
        {
          "name": "adapterUsdcVault",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  102,
                  105,
                  95,
                  97,
                  100,
                  97,
                  112,
                  116,
                  101,
                  114,
                  95,
                  117,
                  115,
                  100,
                  99
                ]
              },
              {
                "kind": "account",
                "path": "adapter_state.pool",
                "account": "defiAdapterState"
              }
            ]
          }
        },
        {
          "name": "adapterKtokenVault",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  102,
                  105,
                  95,
                  97,
                  100,
                  97,
                  112,
                  116,
                  101,
                  114,
                  95,
                  107,
                  116,
                  111,
                  107,
                  101,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "adapter_state.pool",
                "account": "defiAdapterState"
              }
            ]
          }
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        }
      ],
      "args": [],
      "returns": "u64"
    },
    {
      "name": "initializeAdapter",
      "discriminator": [
        220,
        38,
        219,
        51,
        46,
        10,
        185,
        59
      ],
      "accounts": [
        {
          "name": "coreInvoker",
          "docs": [
            "PDA-as-signer proving the call comes from `poolver-core` (arch",
            "§5.2). The `seeds::program` clause anchors the derivation to",
            "core's program ID; no other caller can mint a matching signature."
          ],
          "signer": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  114,
                  101,
                  95,
                  105,
                  110,
                  118,
                  111,
                  107,
                  101,
                  114
                ]
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                21,
                124,
                210,
                15,
                87,
                165,
                71,
                210,
                184,
                103,
                161,
                100,
                247,
                42,
                86,
                99,
                193,
                2,
                253,
                253,
                33,
                8,
                67,
                5,
                191,
                239,
                18,
                109,
                97,
                46,
                112,
                139
              ]
            }
          }
        },
        {
          "name": "payer",
          "docs": [
            "Pays for both the state account and the two token-account rents.",
            "Core proxies this from the pool creator."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "adapterState",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  102,
                  105,
                  95,
                  97,
                  100,
                  97,
                  112,
                  116,
                  101,
                  114
                ]
              },
              {
                "kind": "arg",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "usdcMint",
          "docs": [
            "USDC mint. Anchor's runtime mint check is sufficient — we don't",
            "pin the mint pubkey into the program (same trade as",
            "`poolver-yield-vault`)."
          ]
        },
        {
          "name": "adapterUsdcVault",
          "docs": [
            "PDA-owned LIQUID USDC vault (the 25% buffer). Authority = the",
            "token-account itself; its own seeds sign for transfers."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  102,
                  105,
                  95,
                  97,
                  100,
                  97,
                  112,
                  116,
                  101,
                  114,
                  95,
                  117,
                  115,
                  100,
                  99
                ]
              },
              {
                "kind": "arg",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "adapterKtokenVault",
          "docs": [
            "PDA-owned ktoken vault (mocked as a USDC token account in V1 —",
            "SPEC_QUESTION-19). Authority = the token-account itself.",
            "In production this account's `mint` would be the Kamino kToken",
            "mint; the seed binding stays."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  102,
                  105,
                  95,
                  97,
                  100,
                  97,
                  112,
                  116,
                  101,
                  114,
                  95,
                  107,
                  116,
                  111,
                  107,
                  101,
                  110
                ]
              },
              {
                "kind": "arg",
                "path": "pool"
              }
            ]
          }
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        },
        {
          "name": "rent",
          "address": "SysvarRent111111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "pool",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "mockInjectYield",
      "discriminator": [
        128,
        238,
        10,
        73,
        184,
        0,
        240,
        107
      ],
      "accounts": [
        {
          "name": "injector",
          "docs": [
            "SPEC_QUESTION-26: any signer in V1."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "adapterState",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  102,
                  105,
                  95,
                  97,
                  100,
                  97,
                  112,
                  116,
                  101,
                  114
                ]
              },
              {
                "kind": "account",
                "path": "adapter_state.pool",
                "account": "defiAdapterState"
              }
            ]
          }
        },
        {
          "name": "injectorUsdc",
          "docs": [
            "Source of the injected USDC. Must be authority-owned by",
            "`injector` (the SPL transfer enforces it)."
          ],
          "writable": true
        },
        {
          "name": "adapterKtokenVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  102,
                  105,
                  95,
                  97,
                  100,
                  97,
                  112,
                  116,
                  101,
                  114,
                  95,
                  107,
                  116,
                  111,
                  107,
                  101,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "adapter_state.pool",
                "account": "defiAdapterState"
              }
            ]
          }
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "mockSetKaminoPaused",
      "discriminator": [
        245,
        198,
        251,
        188,
        127,
        83,
        160,
        36
      ],
      "accounts": [
        {
          "name": "admin",
          "docs": [
            "SPEC_QUESTION-26: any signer in V1."
          ],
          "signer": true
        },
        {
          "name": "adapterState",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  102,
                  105,
                  95,
                  97,
                  100,
                  97,
                  112,
                  116,
                  101,
                  114
                ]
              },
              {
                "kind": "account",
                "path": "adapter_state.pool",
                "account": "defiAdapterState"
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "paused",
          "type": "bool"
        }
      ]
    },
    {
      "name": "mockSetOracleDeviation",
      "discriminator": [
        246,
        243,
        79,
        147,
        16,
        122,
        202,
        197
      ],
      "accounts": [
        {
          "name": "admin",
          "docs": [
            "SPEC_QUESTION-26: any signer in V1."
          ],
          "signer": true
        },
        {
          "name": "adapterState",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  102,
                  105,
                  95,
                  97,
                  100,
                  97,
                  112,
                  116,
                  101,
                  114
                ]
              },
              {
                "kind": "account",
                "path": "adapter_state.pool",
                "account": "defiAdapterState"
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "bps",
          "type": "u16"
        }
      ]
    },
    {
      "name": "mockSetUtilization",
      "discriminator": [
        152,
        123,
        169,
        76,
        167,
        144,
        143,
        120
      ],
      "accounts": [
        {
          "name": "admin",
          "docs": [
            "SPEC_QUESTION-26: any signer in V1."
          ],
          "signer": true
        },
        {
          "name": "adapterState",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  102,
                  105,
                  95,
                  97,
                  100,
                  97,
                  112,
                  116,
                  101,
                  114
                ]
              },
              {
                "kind": "account",
                "path": "adapter_state.pool",
                "account": "defiAdapterState"
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "bps",
          "type": "u16"
        }
      ]
    },
    {
      "name": "resetCircuitBreaker",
      "docs": [
        "Operator-driven breaker reset. Always present (not feature-gated)."
      ],
      "discriminator": [
        225,
        48,
        84,
        136,
        90,
        146,
        26,
        149
      ],
      "accounts": [
        {
          "name": "admin",
          "docs": [
            "SPEC_QUESTION-26: in production, constrain `admin == protocol_config.admin`.",
            "V1 leaves it open so the hackathon demo + tests don't need to",
            "thread `protocol_config` through the adapter just to clear a",
            "breaker. The breaker is itself a self-DoS mechanism, not a",
            "theft vector — clearing it can't drain funds."
          ],
          "signer": true
        },
        {
          "name": "adapterState",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  102,
                  105,
                  95,
                  97,
                  100,
                  97,
                  112,
                  116,
                  101,
                  114
                ]
              },
              {
                "kind": "account",
                "path": "adapter_state.pool",
                "account": "defiAdapterState"
              }
            ]
          }
        }
      ],
      "args": []
    },
    {
      "name": "withdraw",
      "discriminator": [
        183,
        18,
        70,
        156,
        148,
        109,
        161,
        34
      ],
      "accounts": [
        {
          "name": "coreInvoker",
          "signer": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  114,
                  101,
                  95,
                  105,
                  110,
                  118,
                  111,
                  107,
                  101,
                  114
                ]
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                21,
                124,
                210,
                15,
                87,
                165,
                71,
                210,
                184,
                103,
                161,
                100,
                247,
                42,
                86,
                99,
                193,
                2,
                253,
                253,
                33,
                8,
                67,
                5,
                191,
                239,
                18,
                109,
                97,
                46,
                112,
                139
              ]
            }
          }
        },
        {
          "name": "adapterState",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  102,
                  105,
                  95,
                  97,
                  100,
                  97,
                  112,
                  116,
                  101,
                  114
                ]
              },
              {
                "kind": "account",
                "path": "adapter_state.pool",
                "account": "defiAdapterState"
              }
            ]
          }
        },
        {
          "name": "adapterUsdcVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  102,
                  105,
                  95,
                  97,
                  100,
                  97,
                  112,
                  116,
                  101,
                  114,
                  95,
                  117,
                  115,
                  100,
                  99
                ]
              },
              {
                "kind": "account",
                "path": "adapter_state.pool",
                "account": "defiAdapterState"
              }
            ]
          }
        },
        {
          "name": "destinationUsdc",
          "docs": [
            "Where to send the withdrawn USDC. Core's `claim_winning` /",
            "`liquidate_default` flows pick this; same trade as Tier 0."
          ],
          "writable": true
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "adapterKtokenVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  102,
                  105,
                  95,
                  97,
                  100,
                  97,
                  112,
                  116,
                  101,
                  114,
                  95,
                  107,
                  116,
                  111,
                  107,
                  101,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "adapter_state.pool",
                "account": "defiAdapterState"
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "defiAdapterState",
      "discriminator": [
        95,
        67,
        109,
        44,
        178,
        169,
        245,
        185
      ]
    }
  ],
  "events": [
    {
      "name": "adapterDeposited",
      "discriminator": [
        243,
        41,
        42,
        64,
        92,
        58,
        70,
        56
      ]
    },
    {
      "name": "adapterHarvested",
      "discriminator": [
        168,
        250,
        13,
        95,
        222,
        137,
        135,
        77
      ]
    },
    {
      "name": "adapterInitialized",
      "discriminator": [
        11,
        81,
        24,
        142,
        122,
        120,
        153,
        118
      ]
    },
    {
      "name": "adapterUnwound",
      "discriminator": [
        190,
        52,
        178,
        224,
        56,
        106,
        224,
        215
      ]
    },
    {
      "name": "adapterWithdrew",
      "discriminator": [
        194,
        114,
        84,
        1,
        145,
        140,
        37,
        86
      ]
    },
    {
      "name": "circuitBreakerReset",
      "discriminator": [
        215,
        141,
        199,
        11,
        71,
        140,
        36,
        87
      ]
    },
    {
      "name": "circuitBreakerTripped",
      "discriminator": [
        188,
        9,
        111,
        118,
        136,
        206,
        199,
        65
      ]
    },
    {
      "name": "mockKaminoPausedSet",
      "discriminator": [
        229,
        12,
        3,
        187,
        72,
        121,
        58,
        35
      ]
    },
    {
      "name": "mockOracleDeviationSet",
      "discriminator": [
        240,
        62,
        105,
        165,
        1,
        193,
        34,
        171
      ]
    },
    {
      "name": "mockUtilizationSet",
      "discriminator": [
        175,
        34,
        154,
        19,
        222,
        222,
        220,
        41
      ]
    },
    {
      "name": "mockYieldInjected",
      "discriminator": [
        66,
        152,
        90,
        211,
        45,
        248,
        32,
        234
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "unauthorized",
      "msg": "Caller is not the canonical core_invoker PDA"
    },
    {
      "code": 6001,
      "name": "circuitBreakerTripped",
      "msg": "Adapter is in tripped state — call reset_circuit_breaker before further use"
    },
    {
      "code": 6002,
      "name": "insufficientLiquidity",
      "msg": "Adapter has insufficient liquidity to satisfy the requested withdrawal"
    },
    {
      "code": 6003,
      "name": "mathOverflow",
      "msg": "Arithmetic overflow"
    },
    {
      "code": 6004,
      "name": "invalidAmount",
      "msg": "Amount must be non-zero"
    },
    {
      "code": 6005,
      "name": "notAdmin",
      "msg": "Caller is not the protocol admin (mock + reset only)"
    }
  ],
  "types": [
    {
      "name": "adapterDeposited",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "deployedToKamino",
            "type": "u64"
          },
          {
            "name": "keptLiquid",
            "type": "u64"
          },
          {
            "name": "totalDeposited",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "adapterHarvested",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "yieldAmount",
            "docs": [
              "Realized yield since the last harvest call. For Tier 1 this is",
              "`current_balance − last_recorded_balance`; in V1 with the mock",
              "it's whatever amount was injected via `mock_inject_yield`."
            ],
            "type": "u64"
          },
          {
            "name": "lastRecordedBalance",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "adapterInitialized",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "adapterState",
            "type": "pubkey"
          },
          {
            "name": "usdcVault",
            "type": "pubkey"
          },
          {
            "name": "ktokenVault",
            "type": "pubkey"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "adapterUnwound",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "amountUnwound",
            "type": "u64"
          },
          {
            "name": "fromLiquid",
            "type": "u64"
          },
          {
            "name": "fromKamino",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "adapterWithdrew",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "fromLiquid",
            "type": "u64"
          },
          {
            "name": "fromKamino",
            "type": "u64"
          },
          {
            "name": "totalDeposited",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "circuitBreakerReset",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "previousReason",
            "type": "u8"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "circuitBreakerTripped",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "reason",
            "type": "u8"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "defiAdapterState",
      "docs": [
        "Tier 1 yield-adapter state. Layout fixed by arch §3.9 (target ≈ 251",
        "bytes including Anchor's 8-byte discriminator). Field order MUST",
        "stay stable so a future upgrade can swap the mock-only fields in the",
        "reserved tail for real Kamino account references without account",
        "reallocation. SPEC_QUESTION-19 / Q-20.",
        "",
        "`total_deposited` / `total_deployed_to_kamino` / `liquid_reserved`",
        "are bookkeeping ledgers, not balances. The authoritative USDC",
        "balances live in the two PDA-owned token accounts (`usdc_vault` =",
        "the liquid 25%, `ktoken_vault` = the simulated Kamino position",
        "holding the deployed 75%). We never trust these ledger fields for",
        "solvency checks — INV-21 / spec §9.1."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "docs": [
              "The pool this adapter belongs to (foreign key into `poolver-core`)."
            ],
            "type": "pubkey"
          },
          {
            "name": "usdcVault",
            "docs": [
              "Liquid USDC vault (the 25% kept on-hand for fast withdrawals)."
            ],
            "type": "pubkey"
          },
          {
            "name": "ktokenVault",
            "docs": [
              "\"kToken\" vault. SPEC_QUESTION-19: in the V1 mock this is just a",
              "second USDC token account simulating the Kamino kToken position",
              "(deployed 75%). When real Kamino lands, this stays a kToken",
              "account; the type is forward-compatible."
            ],
            "type": "pubkey"
          },
          {
            "name": "kaminoReserve",
            "docs": [
              "Placeholder for the Kamino reserve account reference.",
              "SPEC_QUESTION-19: in the V1 mock, set to `Pubkey::default()`.",
              "In production, this is the Kamino-Lend reserve account whose",
              "liquidity we supply into."
            ],
            "type": "pubkey"
          },
          {
            "name": "totalDeposited",
            "docs": [
              "Cumulative net deposit ledger. Bumped by `deposit`, decremented",
              "(saturating) by `withdraw` / `emergency_unwind`."
            ],
            "type": "u64"
          },
          {
            "name": "totalDeployedToKamino",
            "docs": [
              "Bookkeeping for the deployed-to-Kamino (75%) leg."
            ],
            "type": "u64"
          },
          {
            "name": "liquidReserved",
            "docs": [
              "Bookkeeping for the liquid (25%) leg."
            ],
            "type": "u64"
          },
          {
            "name": "lastRecordedBalance",
            "docs": [
              "Snapshot of `usdc_vault.amount + ktoken_vault.amount` taken at",
              "the last `harvest()`. The next `harvest()` returns the delta vs.",
              "this baseline. Initialized to 0 in `initialize_adapter`."
            ],
            "type": "u64"
          },
          {
            "name": "tripped",
            "docs": [
              "Circuit-breaker latch. Set by any failing safety check on",
              "`deposit` / `withdraw` / `harvest`; cleared by",
              "`reset_circuit_breaker` (admin-only). While `tripped == true`,",
              "every state-changing instruction except `reset_circuit_breaker`",
              "rejects with `CircuitBreakerTripped` (spec §4 + §5.3)."
            ],
            "type": "bool"
          },
          {
            "name": "trippedAt",
            "docs": [
              "Trip timestamp. 0 ⇔ `tripped == false`."
            ],
            "type": "i64"
          },
          {
            "name": "trippedReason",
            "docs": [
              "Trip reason discriminant; values defined in `constants::TRIP_*`.",
              "`0` ⇔ `tripped == false`. Kept as `u8` so the field is upgrade-",
              "safe; future variants append to the constant set without state",
              "migration."
            ],
            "type": "u8"
          },
          {
            "name": "mockUtilizationBps",
            "type": "u16"
          },
          {
            "name": "mockOracleDeviationBps",
            "type": "u16"
          },
          {
            "name": "mockKaminoPaused",
            "type": "bool"
          },
          {
            "name": "bump",
            "docs": [
              "Stored canonical bump for `DefiAdapterState`. Saves CU vs.",
              "`find_program_address` per arch §4 + INV-29."
            ],
            "type": "u8"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved tail for forward compat. Sized to land the struct near",
              "arch §3.9's 251-byte target (Anchor adds the 8-byte",
              "discriminator on top). Mock fields above eat 5 bytes of the",
              "nominal 64-byte reserve, so 56 left here keeps the total",
              "constant."
            ],
            "type": {
              "array": [
                "u8",
                56
              ]
            }
          }
        ]
      }
    },
    {
      "name": "mockKaminoPausedSet",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "paused",
            "type": "bool"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "mockOracleDeviationSet",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "bps",
            "type": "u16"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "mockUtilizationSet",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "bps",
            "type": "u16"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "mockYieldInjected",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "newKtokenBalance",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    }
  ]
};
