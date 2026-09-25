import re

def fix_braces():
    with open("src/deriv.rs", "r", encoding="utf-8") as f:
        content = f.read()

    # The end of the function looks like:
    #                     }
    #                 }
    #             }
    #         }
    #     }
    # }
    #     }
    # 
    #     eprintln!("\n⚠️ [Multiplex] WebSocket loop ended.");
    #     Ok(())
    # }

    # We want to remove the two extra `}`. The easiest way is to match the exact string at the end.
    
    end_pattern = """                        break;
                    }
                }
            }
        }
    }
}
    }

    eprintln!("\\n⚠️ [Multiplex] WebSocket loop ended.");
    Ok(())
}
"""

    new_end = """                        break;
                    }
                }
            }
        }
    }

    eprintln!("\\n⚠️ [Multiplex] WebSocket loop ended.");
    Ok(())
}
"""
    
    if end_pattern in content:
        content = content.replace(end_pattern, new_end)
        print("Fixed braces!")
    else:
        # Fallback using regex
        content = re.sub(r'\}\s*\}\s*eprintln!\("\\n⚠️ \[Multiplex\] WebSocket loop ended\."\);\s*Ok\(\(\)\)\s*\}', 
                         r'\n    eprintln!("\\n⚠️ [Multiplex] WebSocket loop ended.");\n    Ok(())\n}', content)
        print("Fixed braces with regex!")

    with open("src/deriv.rs", "w", encoding="utf-8") as f:
        f.write(content)

if __name__ == "__main__":
    fix_braces()
