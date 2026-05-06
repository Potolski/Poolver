/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/poolver_yield_vault.json`.
 */
export type PoolverYieldVault = {
  "address": "A3ERUDLAdqdwgqgAoYLftxA6F1QtxSHZYu8DpNDXyyUp",
  "metadata": {
    "name": "poolverYieldVault",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "Poolver V1 Tier 0 yield adapter (no-yield reference implementation)"
  },
  "docs": [
    "Tier 0 yield adapter — holds USDC in a PDA-owned token account, no",
    "external strategy. See spec §5.3 + arch §13 for the common adapter",
    "interface this implements."
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
                  118,
                  97,
                  117,
                  108,
                  116,
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
                "account": "vaultAdapterState"
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
                  118,
                  97,
                  117,
                  108,
                  116,
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
                "account": "vaultAdapterState"
              }
            ]
          }
        },
        {
          "name": "sourceUsdc",
          "docs": [
            "Source of funds. Core passes the pool's PoolUsdcVault here. The",
            "authority signing the SPL transfer is forwarded by the caller; we",
            "don't constrain `source_usdc.owner` because core handles that on",
            "its side (arch §5.1)."
          ],
          "writable": true
        },
        {
          "name": "sourceAuthority",
          "docs": [
            "Authority over `source_usdc`. Required to sign the SPL transfer —",
            "in practice this is the pool USDC vault PDA owned by core. Passed",
            "in raw because Anchor cannot type a foreign-program PDA here.",
            "CPI time; no further validation needed in this adapter."
          ],
          "signer": true
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
                  118,
                  97,
                  117,
                  108,
                  116,
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
                "account": "vaultAdapterState"
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
                  118,
                  97,
                  117,
                  108,
                  116,
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
                "account": "vaultAdapterState"
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
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  118,
                  97,
                  117,
                  108,
                  116,
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
                "account": "vaultAdapterState"
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
                  118,
                  97,
                  117,
                  108,
                  116,
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
                "account": "vaultAdapterState"
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
            "PDA-as-signer proving the call comes from `poolver-core` (arch §5.2).",
            "The `seeds::program` clause anchors the derivation to core's program",
            "ID; no other caller can mint a matching signature."
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
            "Pays for both the state account and the token-account rent. Core",
            "proxies this from the pool creator."
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
                  118,
                  97,
                  117,
                  108,
                  116,
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
            "USDC mint (6 decimals; checked at runtime to avoid baking the mint",
            "pubkey into the program — Anchor's mint constraint is sufficient)."
          ]
        },
        {
          "name": "adapterUsdcVault",
          "docs": [
            "PDA-owned USDC vault. Authority = the token-account itself (so its",
            "own seeds sign for it during `withdraw` / `emergency_unwind`)."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  118,
                  97,
                  117,
                  108,
                  116,
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
                  118,
                  97,
                  117,
                  108,
                  116,
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
                "account": "vaultAdapterState"
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
                  118,
                  97,
                  117,
                  108,
                  116,
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
                "account": "vaultAdapterState"
              }
            ]
          }
        },
        {
          "name": "destinationUsdc",
          "docs": [
            "Where to send the withdrawn USDC. Core's `claim_winning` /",
            "`liquidate_default` flows pick this; we don't constrain the",
            "destination beyond \"is a token account\" — same-mint check is done",
            "by the SPL transfer."
          ],
          "writable": true
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
    }
  ],
  "accounts": [
    {
      "name": "vaultAdapterState",
      "discriminator": [
        116,
        205,
        43,
        167,
        110,
        190,
        198,
        24
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
      "name": "insufficientLiquidity",
      "msg": "Adapter USDC vault has insufficient liquidity for this withdrawal"
    },
    {
      "code": 6002,
      "name": "mathOverflow",
      "msg": "Arithmetic overflow"
    },
    {
      "code": 6003,
      "name": "invalidAmount",
      "msg": "Amount must be non-zero"
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
              "Tier 0 always emits 0 here; the field exists for indexer parity with",
              "Tier 1 where a non-zero realized yield is expected."
            ],
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
      "name": "vaultAdapterState",
      "docs": [
        "Tier 0 yield-adapter state. Layout fixed by arch §3.8 (81 bytes total",
        "including Anchor's 8-byte discriminator). The struct below contributes 73",
        "bytes; Anchor adds the discriminator on top → 81. Field order MUST stay",
        "stable so a future upgrade can be done without account reallocation.",
        "",
        "`total_deposited` is the cumulative net deposit ledger; the authoritative",
        "USDC balance lives in `VaultAdapterUsdc`. We never trust this field for",
        "solvency checks — see INV-21 / spec §9.1 (\"never trust adapter return",
        "values without bounds-checking\")."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "pool",
            "type": "pubkey"
          },
          {
            "name": "usdcVault",
            "type": "pubkey"
          },
          {
            "name": "totalDeposited",
            "type": "u64"
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    }
  ]
};
