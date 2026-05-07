/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/poolver_reserve.json`.
 */
export type PoolverReserve = {
  "address": "CfxRT3jsXWQZRev67ztqaNKCrHaKF6ieW9a1E8NDPvnx",
  "metadata": {
    "name": "poolverReserve",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "Poolver V1 tier-segregated reserve fund (raw USDC custody for default coverage)"
  },
  "docs": [
    "Tier-segregated reserve fund. Holds raw USDC per tier (Vault / DeFi)",
    "and pays out during default-coverage liquidations. Spec §5.2 + arch",
    "§3.5 + §11.",
    "",
    "All mutating instructions except `initialize_reserve` and `seed` are",
    "CPI-only from `poolver-core` (auth via `core_invoker` PDA, arch §5.2)."
  ],
  "instructions": [
    {
      "name": "adminCloseReserve",
      "discriminator": [
        7,
        95,
        80,
        63,
        161,
        170,
        222,
        56
      ],
      "accounts": [
        {
          "name": "caller",
          "docs": [
            "Receives the rent refund from both the `ReserveFund` PDA and the",
            "`reserve_usdc_vault` token account. SPEC_QUESTION-26: V1 accepts",
            "any signer (matches `initialize_reserve`)."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "reserveFund",
          "writable": true
        },
        {
          "name": "reserveUsdcVault",
          "docs": [
            "We don't deserialize as `TokenAccount` because the close CPI inside",
            "the handler invalidates the discriminator before Anchor's drop-time",
            "re-serialise check would run."
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
          "name": "tier",
          "type": {
            "defined": {
              "name": "tier"
            }
          }
        }
      ]
    },
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
          "docs": [
            "PDA-as-signer proving the call comes from `poolver-core` (arch §5.2).",
            "`seeds::program = POOLVER_CORE_ID` anchors the derivation to core's",
            "program ID; no other caller can mint a matching signature."
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
          "name": "reserveFund",
          "docs": [
            "The reserve fund itself. INV-4: tier comes from the seed, not from",
            "instruction args. Caller passing the wrong-tier reserve gets",
            "`ConstraintSeeds`; this is the structural enforcement promised by",
            "arch §11."
          ],
          "writable": true
        },
        {
          "name": "reserveUsdcVault",
          "docs": [
            "PDA-owned USDC vault for this tier. Same tier-encoded seed."
          ],
          "writable": true
        },
        {
          "name": "sourceUsdc",
          "docs": [
            "Source of funds. Core passes the pool's `PoolUsdcVault` (or another",
            "core-controlled token account) here. The authority signing the SPL",
            "transfer is forwarded as `source_authority`."
          ],
          "writable": true
        },
        {
          "name": "sourceAuthority",
          "docs": [
            "Authority over `source_usdc`. Required to sign the SPL transfer —",
            "in practice this is a core-owned PDA (e.g. the pool USDC vault PDA).",
            "We don't constrain this further because core handles ownership on",
            "its side (arch §5.1)."
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
      "name": "draw",
      "discriminator": [
        61,
        40,
        62,
        184,
        31,
        176,
        24,
        130
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
          "name": "reserveFund",
          "writable": true
        },
        {
          "name": "reserveUsdcVault",
          "writable": true
        },
        {
          "name": "destinationUsdc",
          "docs": [
            "Where to send the drawn USDC. Core's `liquidate_default` flow",
            "chooses this; we don't constrain the destination beyond \"is a token",
            "account\" — the SPL transfer enforces same-mint."
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
    },
    {
      "name": "initializeReserve",
      "discriminator": [
        91,
        188,
        92,
        135,
        153,
        155,
        112,
        16
      ],
      "accounts": [
        {
          "name": "admin",
          "docs": [
            "Pays for both the state account and the token-account rent.",
            "SPEC_QUESTION-26: this signer becomes \"must equal ProtocolConfig.admin\"",
            "once core lands."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "reserveFund",
          "writable": true
        },
        {
          "name": "usdcMint",
          "docs": [
            "USDC mint. The constraint that this is in fact the canonical USDC",
            "mint will land with `poolver-core::ProtocolConfig` — until then we",
            "accept whatever mint admin passes (the tests rely on this for the",
            "fake-USDC fixture).",
            "SPEC_QUESTION-26: validate against `ProtocolConfig.usdc_mint` once",
            "core lands."
          ]
        },
        {
          "name": "reserveUsdcVault",
          "docs": [
            "PDA-owned USDC vault. Authority = the token account itself; its own",
            "seeds sign for it during `draw` (matches the yield-vault pattern)."
          ],
          "writable": true
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
          "name": "tier",
          "type": {
            "defined": {
              "name": "tier"
            }
          }
        }
      ]
    },
    {
      "name": "seed",
      "discriminator": [
        255,
        178,
        140,
        239,
        113,
        22,
        214,
        231
      ],
      "accounts": [
        {
          "name": "funder",
          "docs": [
            "Admin (or, in V1, anyone) topping up the reserve.",
            "SPEC_QUESTION-26: tighten to ProtocolConfig.admin when core lands."
          ],
          "signer": true
        },
        {
          "name": "reserveFund",
          "writable": true
        },
        {
          "name": "reserveUsdcVault",
          "writable": true
        },
        {
          "name": "sourceUsdc",
          "docs": [
            "Source USDC. Owned/signed by `funder`."
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
      "name": "reserveFund",
      "discriminator": [
        178,
        39,
        73,
        74,
        131,
        247,
        6,
        163
      ]
    }
  ],
  "events": [
    {
      "name": "reserveClosed",
      "discriminator": [
        163,
        73,
        88,
        57,
        0,
        150,
        89,
        217
      ]
    },
    {
      "name": "reserveDeposit",
      "discriminator": [
        199,
        40,
        176,
        78,
        52,
        0,
        184,
        137
      ]
    },
    {
      "name": "reserveDraw",
      "discriminator": [
        198,
        105,
        22,
        137,
        0,
        213,
        228,
        160
      ]
    },
    {
      "name": "reserveInitialized",
      "discriminator": [
        22,
        27,
        136,
        173,
        244,
        120,
        20,
        49
      ]
    },
    {
      "name": "reserveSeeded",
      "discriminator": [
        193,
        145,
        90,
        8,
        5,
        2,
        185,
        73
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
      "name": "reserveInsufficient",
      "msg": "Reserve has insufficient balance to satisfy the draw amount"
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
      "name": "reserveClosed",
      "docs": [
        "SPEC_QUESTION-26: emitted by `admin_close_reserve` when the tier reserve",
        "is torn down ahead of a re-`initialize_reserve` with a different USDC",
        "mint. Indexers should treat this as \"tier reserve rotation in progress\";",
        "a fresh `ReserveInitialized(tier)` will follow."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tier",
            "type": {
              "defined": {
                "name": "tier"
              }
            }
          },
          {
            "name": "reserveFund",
            "type": "pubkey"
          },
          {
            "name": "reserveUsdcVault",
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
      "name": "reserveDeposit",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tier",
            "type": {
              "defined": {
                "name": "tier"
              }
            }
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "totalBalance",
            "type": "u64"
          },
          {
            "name": "totalInflows",
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
      "name": "reserveDraw",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tier",
            "type": {
              "defined": {
                "name": "tier"
              }
            }
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "totalBalance",
            "type": "u64"
          },
          {
            "name": "totalOutflows",
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
      "name": "reserveFund",
      "docs": [
        "Reserve fund state. Layout fixed by arch §3.5 (98 bytes total including",
        "Anchor's 8-byte discriminator). Field order MUST stay stable so a future",
        "upgrade can be done without account reallocation.",
        "",
        "Three monotonic invariants live on this struct:",
        "- INV-2: `total_balance >= 0` (enforced via `checked_sub` in `draw`).",
        "- INV-3: `total_balance == total_inflows − total_outflows` at all times.",
        "- INV-4: tier isolation is structural — see arch §11 / `Tier` enum.",
        "",
        "`total_inflows` and `total_outflows` are lifetime counters; they NEVER",
        "decrease.  Every reserve mutation re-establishes the inflow/outflow",
        "identity post-update."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tier",
            "type": {
              "defined": {
                "name": "tier"
              }
            }
          },
          {
            "name": "totalBalance",
            "type": "u64"
          },
          {
            "name": "totalInflows",
            "type": "u64"
          },
          {
            "name": "totalOutflows",
            "type": "u64"
          },
          {
            "name": "usdcVault",
            "type": "pubkey"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved padding for future fields without account reallocation;",
              "matches arch §3.5's 32-byte `_reserved` block."
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          }
        ]
      }
    },
    {
      "name": "reserveInitialized",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tier",
            "type": {
              "defined": {
                "name": "tier"
              }
            }
          },
          {
            "name": "reserveFund",
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
      "name": "reserveSeeded",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tier",
            "type": {
              "defined": {
                "name": "tier"
              }
            }
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "totalBalance",
            "type": "u64"
          },
          {
            "name": "totalInflows",
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
      "name": "tier",
      "docs": [
        "Tier discriminant. The on-wire byte equals `tier as u8`; the same byte",
        "is used as the PDA seed suffix, so the `Tier` enum and the",
        "`(tier as u8).to_le_bytes()` derivation must stay aligned forever.",
        "",
        "`repr(u8)` plus explicit discriminants pin the byte values across",
        "compiler versions (INV-4: structural tier isolation depends on these",
        "bytes never drifting).",
        "Borsh tag bytes are assigned by source order: `Vault = 0`, `DeFi = 1`.",
        "This MUST never be reordered — the same byte is used as the PDA seed",
        "suffix (INV-4 isolation depends on tier-byte stability). The",
        "`tier_byte` constants below + INV-4 tests guard against drift."
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "vault"
          },
          {
            "name": "deFi"
          }
        ]
      }
    }
  ]
};
